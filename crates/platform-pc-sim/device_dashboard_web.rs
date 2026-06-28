#[path = "device_dashboard_web/flash.rs"]
mod flash;
#[path = "device_dashboard_web/http_util.rs"]
mod http_util;
#[path = "device_dashboard_web/sim_rig.rs"]
mod sim_rig;

use std::collections::VecDeque;
use std::env;
use std::io::{self, Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use platform_pc_sim::dashboard::BoardProfile;
use platform_pc_sim::web_dashboard::{dashboard_html, join_diag_events, state_to_json};
use platform_pc_sim::wiring_config::{
    normalize_supported_device_selection, DeviceKind, SensorProfile, WiringConfig,
};
use platform_pc_sim::wiring_svg::wiring_svg;

use flash::{flash_targets, handle_flash_stream, list_serial_ports};
use http_util::{
    parse_board_from_json, parse_json_bool_field, parse_json_string_array_field,
    parse_sensor_profile_from_json, respond,
};
use sim_rig::DeviceSimulationRig;

// Items only needed in the test module — pulled into test scope via `use super::*`.
#[cfg(test)]
use flash::{board_kind_from_str, detect_binary_name, detect_build_target, BoardKind};
#[cfg(test)]
use hal_api::actuator::MotorDirection;
#[cfg(test)]
use http_util::parse_json_string_field;
#[cfg(test)]
use sim_rig::{blank_lines, distance_to_servo_angle, motor_commands_from_state};

const DEFAULT_PORT: u16 = 7878;

/// Combined board + sensor profile state read/written as a unit.
#[derive(Clone)]
struct WiringState {
    board: BoardProfile,
    sensor_profile: SensorProfile,
    selected_devices: Vec<DeviceKind>,
    show_bus_labels: bool,
}

fn dashboard_wiring_config(
    board: BoardProfile,
    sensor_profile: SensorProfile,
    selected_devices: &[DeviceKind],
    show_bus_labels: bool,
) -> WiringConfig {
    WiringConfig::from_board_with_selected_devices(board, sensor_profile, selected_devices)
        .with_bus_labels(show_bus_labels)
}

/// Ring buffer holding the most recent N sensor readings for history charts.
struct SensorHistoryBuffer {
    /// (temperature_centi_celsius, humidity_centi_percent, pressure_pascal)
    climate: VecDeque<(i32, u32, Option<u32>)>,
    /// distance_mm
    distance: VecDeque<Option<u32>>,
    capacity: usize,
}

impl SensorHistoryBuffer {
    fn new(capacity: usize) -> Self {
        Self {
            climate: VecDeque::with_capacity(capacity),
            distance: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    fn push_climate(&mut self, temp_cc: i32, hum_cp: u32, pressure_pa: Option<u32>) {
        if self.climate.len() >= self.capacity {
            self.climate.pop_front();
        }
        self.climate.push_back((temp_cc, hum_cp, pressure_pa));
    }

    fn push_distance(&mut self, distance_mm: Option<u32>) {
        if self.distance.len() >= self.capacity {
            self.distance.pop_front();
        }
        self.distance.push_back(distance_mm);
    }

    fn climate_json(&self) -> String {
        let temp: Vec<String> = self
            .climate
            .iter()
            .map(|(t, _, _)| format!("{:.2}", *t as f32 / 100.0))
            .collect();
        let hum: Vec<String> = self
            .climate
            .iter()
            .map(|(_, h, _)| format!("{:.2}", *h as f32 / 100.0))
            .collect();
        let press: Vec<String> = self
            .climate
            .iter()
            .map(|(_, _, p)| {
                p.map(|v| v.to_string())
                    .unwrap_or_else(|| "null".to_string())
            })
            .collect();
        format!(
            "{{\"temperature\":[{}],\"humidity\":[{}],\"pressure\":[{}]}}",
            temp.join(","),
            hum.join(","),
            press.join(",")
        )
    }

    fn distance_json(&self) -> String {
        let vals: Vec<String> = self
            .distance
            .iter()
            .map(|d| {
                d.map(|v| v.to_string())
                    .unwrap_or_else(|| "null".to_string())
            })
            .collect();
        format!("{{\"distance\":[{}]}}", vals.join(","))
    }
}

/// Shared server state passed to every connection-handler thread.
struct ServerContext {
    latest_json: Mutex<String>,
    sse_clients: Mutex<Vec<mpsc::SyncSender<String>>>,
    current_board: Mutex<BoardProfile>,
    /// Kept for future use; wiring API reads now go through `wiring_state`.
    #[allow(dead_code)]
    current_sensor_profile: Mutex<SensorProfile>,
    /// Atomic board + sensor profile state for wiring API reads.
    wiring_state: Mutex<WiringState>,
    /// Last wiring-editor JSON submitted via POST /api/wiring/editor.
    editor_json: Mutex<String>,
    /// Snapshot of diagnostics ring from the last sim tick (for /api/diagnostics).
    latest_diagnostics: Mutex<String>,
    /// Ring buffer of sensor readings for /api/history.
    history: Mutex<SensorHistoryBuffer>,
}

impl ServerContext {
    fn new(board: BoardProfile) -> Arc<Self> {
        Arc::new(Self {
            latest_json: Mutex::new("{}".into()),
            sse_clients: Mutex::new(vec![]),
            current_board: Mutex::new(board),
            current_sensor_profile: Mutex::new(SensorProfile::Full),
            wiring_state: Mutex::new(WiringState {
                board,
                sensor_profile: SensorProfile::Full,
                selected_devices: normalize_supported_device_selection(
                    board,
                    &SensorProfile::Full.device_kinds(),
                ),
                show_bus_labels: false,
            }),
            editor_json: Mutex::new("{}".into()),
            latest_diagnostics: Mutex::new("[]".into()),
            history: Mutex::new(SensorHistoryBuffer::new(300)),
        })
    }

    fn push_state(&self, json: String, diag_json: String) {
        *self.latest_json.lock().unwrap() = json.clone();
        *self.latest_diagnostics.lock().unwrap() = diag_json;
        self.sse_clients
            .lock()
            .unwrap()
            .retain(|tx| tx.try_send(json.clone()).is_ok());
    }
}

#[cfg(test)]
fn parse_selected_devices_from_json(json: &str) -> Vec<String> {
    parse_json_string_array_field(json, "selected_devices").unwrap_or_default()
}

/// `data: {line}\n\n`.  A final `data: [DONE] exit=N\n\n` closes the stream.
fn handle_test_stream(stream: &mut TcpStream) {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};
    use std::sync::mpsc;

    let header = "HTTP/1.1 200 OK\r\n\
        Content-Type: text/event-stream\r\n\
        Cache-Control: no-cache\r\n\
        Connection: keep-alive\r\n\
        Access-Control-Allow-Origin: *\r\n\
        \r\n";
    if stream.write_all(header.as_bytes()).is_err() {
        return;
    }

    let mut child = match Command::new("cargo")
        .args(["test", "--workspace", "--color=never"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            let _ = stream.write_all(
                format!("data: [ERROR] failed to spawn: {e}\n\ndata: [DONE] exit=1\n\n").as_bytes(),
            );
            return;
        }
    };

    let (tx, rx) = mpsc::channel::<String>();
    let tx_out = tx.clone();
    let stdout = child.stdout.take().expect("stdout piped");
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if tx_out.send(line).is_err() {
                break;
            }
        }
    });
    let tx_err = tx.clone();
    let stderr = child.stderr.take().expect("stderr piped");
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            if tx_err.send(line).is_err() {
                break;
            }
        }
    });
    drop(tx);

    let started = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(300);

    for line in rx {
        if started.elapsed() > timeout {
            let _ = stream.write_all(
                b"data: [ERROR] timeout: cargo test exceeded 5 minutes\n\ndata: [DONE] exit=1\n\n",
            );
            let _ = child.kill();
            return;
        }
        let msg = format!("data: {}\n\n", line.replace('\n', " "));
        if stream.write_all(msg.as_bytes()).is_err() {
            let _ = child.kill();
            return;
        }
    }

    let exit_code = child.wait().map(|s| s.code().unwrap_or(-1)).unwrap_or(-1);
    let _ = stream.write_all(format!("data: [DONE] exit={exit_code}\n\n").as_bytes());
}

fn main() {
    let mut args = env::args().skip(1);
    let first = args.next();
    let second = args.next();
    let board = BoardProfile::from_arg(first.as_deref());
    let port = second
        .as_deref()
        .and_then(|value| value.parse::<u16>().ok())
        .or_else(|| first.as_deref().and_then(|value| value.parse::<u16>().ok()))
        .unwrap_or(DEFAULT_PORT);

    let listener = TcpListener::bind(("127.0.0.1", port)).expect("server should bind");
    listener
        .set_nonblocking(true)
        .expect("non-blocking should be supported");

    let ctx = ServerContext::new(board);
    let (board_tx, board_rx) = mpsc::channel::<BoardProfile>();
    let mut rig = DeviceSimulationRig::new(board);
    let mut push_ticker: u32 = 0;

    println!("device dashboard server started");
    println!("open http://127.0.0.1:{port}");
    println!("board profile: {}", board.name());
    println!("SSE endpoint: http://127.0.0.1:{port}/api/events");

    loop {
        // Apply pending board change from a handler thread.
        if let Ok(new_board) = board_rx.try_recv() {
            rig = DeviceSimulationRig::new(new_board);
            *ctx.current_board.lock().unwrap() = new_board;
            println!("board changed to: {}", new_board.name());
        }

        // Tick the simulation.
        let wiring_state = ctx.wiring_state.lock().unwrap().clone();
        let state = rig.step(&wiring_state);
        push_ticker = push_ticker.wrapping_add(1);

        // Push JSON to SSE clients every 10 ticks (~100 ms).
        if push_ticker % 10 == 0 {
            let diag = &state.diagnostics;
            let events_json = join_diag_events(&diag.recent_events);
            let diag_json = format!(
                "{{\"event_count\":{},\"recent_events\":[{}]}}",
                diag.event_count, events_json
            );
            // Append to history ring buffers when sensors are active.
            {
                let mut hist = ctx.history.lock().unwrap();
                let cli = &state.climate;
                if cli.temperature_c.is_some() || cli.humidity_percent.is_some() {
                    let temp_cc = cli.temperature_c.map(|v| (v * 100.0) as i32).unwrap_or(0);
                    let hum_cp = cli
                        .humidity_percent
                        .map(|v| (v * 100.0) as u32)
                        .unwrap_or(0);
                    hist.push_climate(temp_cc, hum_cp, cli.pressure_pa);
                }
                hist.push_distance(state.distance.distance_mm);
            }
            ctx.push_state(state_to_json(&state), diag_json);
        }

        // Accept new TCP connections (non-blocking).
        loop {
            match listener.accept() {
                Ok((stream, _)) => {
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                    let ctx_clone = Arc::clone(&ctx);
                    let board_tx_clone = board_tx.clone();
                    thread::spawn(move || {
                        handle_connection(stream, ctx_clone, board_tx_clone);
                    });
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }

        thread::sleep(Duration::from_millis(10));
    }
}

fn handle_connection(
    mut stream: TcpStream,
    ctx: Arc<ServerContext>,
    board_tx: mpsc::Sender<BoardProfile>,
) {
    let mut request_buf = [0u8; 4096];
    let Ok(read_len) = stream.read(&mut request_buf) else {
        return;
    };
    let raw = &request_buf[..read_len];
    let request = String::from_utf8_lossy(raw);

    let first_line = request.lines().next().unwrap_or("GET / HTTP/1.1");
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or("GET");
    let path = parts.next().unwrap_or("/");
    let (path_only, query_str) = path.split_once('?').unwrap_or((path, ""));

    let body = request
        .find("\r\n\r\n")
        .map(|pos| &request[pos + 4..])
        .unwrap_or("");

    match (method, path_only) {
        (_, "/") => respond(
            &mut stream,
            "200 OK",
            "text/html; charset=utf-8",
            dashboard_html(),
        ),
        (_, "/api/events") => {
            handle_sse_events(&mut stream, ctx);
        }
        (_, "/api/state") => {
            let json = ctx.latest_json.lock().unwrap().clone();
            respond(
                &mut stream,
                "200 OK",
                "application/json; charset=utf-8",
                &json,
            );
        }
        ("POST", "/api/wiring") => {
            if let Some(board_name) = parse_board_from_json(body) {
                let new_board = BoardProfile::from_arg(Some(board_name));
                let _ = board_tx.send(new_board);
                // Give the main thread time to apply the change.
                thread::sleep(Duration::from_millis(50));
            }
            // Update wiring_state atomically as a single unit.
            let wiring = {
                let mut ws = ctx.wiring_state.lock().unwrap();
                if let Some(board_name) = parse_board_from_json(body) {
                    ws.board = BoardProfile::from_arg(Some(board_name));
                }
                if let Some(profile_slug) = parse_sensor_profile_from_json(body) {
                    if let Some(profile) = SensorProfile::from_slug(profile_slug) {
                        ws.sensor_profile = profile;
                        ws.selected_devices = profile.device_kinds();
                    }
                }
                if let Some(selected_devices) =
                    parse_json_string_array_field(body, "selected_devices")
                {
                    ws.selected_devices = selected_devices
                        .into_iter()
                        .filter_map(|slug| DeviceKind::from_slug(&slug))
                        .collect();
                }
                if let Some(show_bus_labels) = parse_json_bool_field(body, "show_bus_labels") {
                    ws.show_bus_labels = show_bus_labels;
                }
                ws.selected_devices =
                    normalize_supported_device_selection(ws.board, &ws.selected_devices);
                ws.clone()
            };
            let payload = dashboard_wiring_config(
                wiring.board,
                wiring.sensor_profile,
                &wiring.selected_devices,
                wiring.show_bus_labels,
            )
            .to_json();
            respond(
                &mut stream,
                "200 OK",
                "application/json; charset=utf-8",
                &payload,
            );
        }
        (_, "/api/wiring/profiles") => {
            let entries: Vec<String> = SensorProfile::all_variants()
                .iter()
                .map(|p| {
                    let devices = p
                        .device_kinds()
                        .into_iter()
                        .map(|kind| format!(r#""{}""#, kind.slug()))
                        .collect::<Vec<_>>()
                        .join(",");
                    format!(
                        r#"{{"slug":"{}","name":"{}","devices":[{}]}}"#,
                        p.slug(),
                        p.display_name(),
                        devices
                    )
                })
                .collect();
            let payload = format!(r#"{{"profiles":[{}]}}"#, entries.join(","));
            respond(
                &mut stream,
                "200 OK",
                "application/json; charset=utf-8",
                &payload,
            );
        }
        (_, "/api/wiring") => {
            let wiring = ctx.wiring_state.lock().unwrap().clone();
            let payload = dashboard_wiring_config(
                wiring.board,
                wiring.sensor_profile,
                &wiring.selected_devices,
                wiring.show_bus_labels,
            )
            .to_json();
            respond(
                &mut stream,
                "200 OK",
                "application/json; charset=utf-8",
                &payload,
            );
        }
        (_, "/api/wiring/svg") => {
            let wiring = ctx.wiring_state.lock().unwrap().clone();
            let cfg = dashboard_wiring_config(
                wiring.board,
                wiring.sensor_profile,
                &wiring.selected_devices,
                wiring.show_bus_labels,
            );
            let svg = wiring_svg(&cfg);
            respond(&mut stream, "200 OK", "image/svg+xml; charset=utf-8", &svg);
        }
        ("POST", "/api/wiring/editor") => {
            *ctx.editor_json.lock().unwrap() = body.to_string();
            respond(
                &mut stream,
                "200 OK",
                "application/json; charset=utf-8",
                r#"{"ok":true}"#,
            );
        }
        (_, "/api/wiring/editor") => {
            let json = ctx.editor_json.lock().unwrap().clone();
            respond(
                &mut stream,
                "200 OK",
                "application/json; charset=utf-8",
                &json,
            );
        }
        (_, "/api/diagnostics") => {
            let json = ctx.latest_diagnostics.lock().unwrap().clone();
            respond(
                &mut stream,
                "200 OK",
                "application/json; charset=utf-8",
                &json,
            );
        }
        (_, "/api/history") => {
            let sensor = query_str
                .split('&')
                .find_map(|part| {
                    let (k, v) = part.split_once('=')?;
                    if k == "sensor" {
                        Some(v)
                    } else {
                        None
                    }
                })
                .unwrap_or("bme280");
            let json = {
                let hist = ctx.history.lock().unwrap();
                match sensor {
                    "distance" => hist.distance_json(),
                    _ => hist.climate_json(),
                }
            };
            respond(
                &mut stream,
                "200 OK",
                "application/json; charset=utf-8",
                &json,
            );
        }
        (_, "/api/test/stream") => {
            handle_test_stream(&mut stream);
        }
        (_, "/api/flash/devices") => {
            let ports = list_serial_ports();
            let json = format!(
                "[{}]",
                ports
                    .iter()
                    .map(|p| format!("\"{}\"", p))
                    .collect::<Vec<_>>()
                    .join(",")
            );
            respond(
                &mut stream,
                "200 OK",
                "application/json; charset=utf-8",
                &json,
            );
        }
        (_, "/api/flash/targets") => {
            let entries: Vec<String> = flash_targets()
                .iter()
                .map(|t| {
                    format!(
                        "{{\"id\":\"{}\",\"label\":\"{}\",\"board\":\"{}\"}}",
                        t.id,
                        t.label,
                        t.board.label()
                    )
                })
                .collect();
            let json = format!("[{}]", entries.join(","));
            respond(
                &mut stream,
                "200 OK",
                "application/json; charset=utf-8",
                &json,
            );
        }
        (_, "/api/flash/stream") => {
            handle_flash_stream(&mut stream, query_str);
        }
        _ => respond(
            &mut stream,
            "404 Not Found",
            "text/plain; charset=utf-8",
            "not found",
        ),
    }
}

fn handle_sse_events(stream: &mut TcpStream, ctx: Arc<ServerContext>) {
    let header = "HTTP/1.1 200 OK\r\n\
        Content-Type: text/event-stream\r\n\
        Cache-Control: no-cache\r\n\
        Connection: keep-alive\r\n\
        Access-Control-Allow-Origin: *\r\n\
        \r\n";
    if stream.write_all(header.as_bytes()).is_err() {
        return;
    }

    let initial = ctx.latest_json.lock().unwrap().clone();
    let (tx, rx) = mpsc::sync_channel::<String>(32);
    ctx.sse_clients.lock().unwrap().push(tx);

    if stream
        .write_all(format!("data: {initial}\n\n").as_bytes())
        .is_err()
    {
        return;
    }

    for json in rx {
        if stream
            .write_all(format!("data: {json}\n\n").as_bytes())
            .is_err()
        {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Shutdown, TcpListener, TcpStream};
    use std::sync::Arc;
    use std::time::Duration;

    fn read_response(stream: &mut TcpStream) -> String {
        let mut buf = Vec::new();
        let mut chunk = [0u8; 1024];

        loop {
            match stream.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => buf.extend_from_slice(&chunk[..n]),
                Err(err)
                    if matches!(
                        err.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                    ) =>
                {
                    break
                }
                Err(err) => panic!("failed to read response: {err}"),
            }
        }

        String::from_utf8(buf).expect("response should be valid utf-8")
    }

    fn send_request(addr: std::net::SocketAddr, request: &str) -> String {
        let mut client = TcpStream::connect(addr).expect("client should connect");
        client
            .set_read_timeout(Some(Duration::from_millis(200)))
            .expect("client read timeout should be set");
        client
            .write_all(request.as_bytes())
            .expect("request should be written");
        read_response(&mut client)
    }

    #[test]
    fn distance_to_servo_angle_at_minimum() {
        assert_eq!(distance_to_servo_angle(80), 0);
    }

    #[test]
    fn distance_to_servo_angle_at_maximum() {
        assert_eq!(distance_to_servo_angle(360), 180);
    }

    #[test]
    fn distance_to_servo_angle_at_midpoint() {
        // 220mm → clamped = 220 - 80 = 140 → (140 * 180) / 280 = 90
        let angle = distance_to_servo_angle(220);
        assert!(angle > 60 && angle < 120, "expected ~90, got {angle}");
    }

    #[test]
    fn motor_commands_from_state_reverses_when_obstacle_close() {
        // distance < 160 → both channels Reverse
        let (left, right) = motor_commands_from_state(Some(100), None);
        assert_eq!(left.direction, MotorDirection::Reverse);
        assert_eq!(right.direction, MotorDirection::Reverse);
    }

    #[test]
    fn motor_commands_from_state_drives_forward_when_clear() {
        // distance = 200 >= 160, no IMU tilt → both channels Forward straight
        let (left, right) = motor_commands_from_state(Some(200), None);
        assert_eq!(left.direction, MotorDirection::Forward);
        assert_eq!(right.direction, MotorDirection::Forward);
        assert_eq!(left.duty_percent, right.duty_percent);
    }

    #[test]
    fn parse_board_from_json_extracts_board_name() {
        assert_eq!(
            parse_board_from_json(r#"{"board":"arduino-nano"}"#),
            Some("arduino-nano")
        );
    }

    #[test]
    fn parse_board_from_json_handles_esp32_value() {
        assert_eq!(
            parse_board_from_json(r#"{"board":"original-esp32"}"#),
            Some("original-esp32")
        );
    }

    #[test]
    fn parse_board_from_json_returns_none_for_missing_key() {
        assert_eq!(parse_board_from_json(r#"{"other":"value"}"#), None);
    }

    #[test]
    fn parse_board_from_json_returns_none_for_empty_body() {
        assert_eq!(parse_board_from_json(""), None);
    }

    #[test]
    fn parse_sensor_profile_from_json_extracts_profile() {
        assert_eq!(
            parse_sensor_profile_from_json(r#"{"sensor_profile":"climate"}"#),
            Some("climate")
        );
    }

    #[test]
    fn parse_sensor_profile_from_json_handles_combined_body() {
        assert_eq!(
            parse_sensor_profile_from_json(r#"{"board":"esp32","sensor_profile":"robot"}"#),
            Some("robot")
        );
    }

    #[test]
    fn parse_sensor_profile_from_json_returns_none_for_missing_key() {
        assert_eq!(parse_sensor_profile_from_json(r#"{"board":"esp32"}"#), None);
    }

    #[test]
    fn parse_sensor_profile_from_json_returns_none_for_empty_body() {
        assert_eq!(parse_sensor_profile_from_json(""), None);
    }

    #[test]
    fn parse_selected_devices_from_json_extracts_values() {
        assert_eq!(
            parse_selected_devices_from_json(r#"{"selected_devices":["bme280","servo","sgp30"]}"#),
            vec![
                "bme280".to_string(),
                "servo".to_string(),
                "sgp30".to_string()
            ]
        );
    }

    #[test]
    fn parse_json_string_field_handles_space_after_colon() {
        assert_eq!(
            parse_json_string_field(r#"{"sensor_profile": "climate"}"#, "sensor_profile"),
            Some("climate")
        );
    }

    #[test]
    fn sse_events_endpoint_streams_initial_state() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("listener should bind");
        let addr = listener
            .local_addr()
            .expect("listener should have local addr");
        let ctx = ServerContext::new(BoardProfile::OriginalEsp32);
        *ctx.latest_json.lock().unwrap() = r#"{"tick":1}"#.to_string();

        let (board_tx, board_rx) = mpsc::channel::<BoardProfile>();
        drop(board_rx);

        let ctx_for_thread = Arc::clone(&ctx);
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("test client should connect");
            handle_connection(stream, ctx_for_thread, board_tx);
        });

        let mut client = TcpStream::connect(addr).expect("client should connect");
        client
            .set_read_timeout(Some(Duration::from_millis(200)))
            .expect("client read timeout should be set");
        client
            .write_all(b"GET /api/events HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .expect("request should be written");

        let response = read_response(&mut client);
        assert!(response.contains("HTTP/1.1 200 OK\r\n"));
        assert!(response.contains("Content-Type: text/event-stream\r\n"));
        assert!(response.contains("data: {\"tick\":1}\n\n"));

        client
            .shutdown(Shutdown::Both)
            .expect("client should shut down cleanly");
        ctx.sse_clients.lock().unwrap().clear();

        server.join().expect("server thread should exit");
    }

    #[test]
    fn wiring_endpoint_updates_selected_devices() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("listener should bind");
        let addr = listener
            .local_addr()
            .expect("listener should have local addr");
        let ctx = ServerContext::new(BoardProfile::OriginalEsp32);

        let (board_tx, board_rx) = mpsc::channel::<BoardProfile>();
        drop(board_rx);

        let ctx_for_thread = Arc::clone(&ctx);
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("test client should connect");
            handle_connection(stream, ctx_for_thread, board_tx);
        });

        let body = r#"{"sensor_profile":"minimal","selected_devices":["bme280","servo"]}"#;
        let request = format!(
            "POST /api/wiring HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );

        let mut client = TcpStream::connect(addr).expect("client should connect");
        client
            .set_read_timeout(Some(Duration::from_millis(200)))
            .expect("client read timeout should be set");
        client
            .write_all(request.as_bytes())
            .expect("request should be written");

        let response = read_response(&mut client);
        assert!(response.contains("\"sensor_profile\":\"minimal\""));
        assert!(response.contains("\"selected_devices\":[\"bme280\",\"servo\"]"));
        assert!(response.contains("\"available_devices\":["));

        server.join().expect("server thread should exit");
    }

    #[test]
    fn wiring_endpoint_persists_bus_label_toggle_and_svg() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("listener should bind");
        let addr = listener
            .local_addr()
            .expect("listener should have local addr");
        let ctx = ServerContext::new(BoardProfile::OriginalEsp32);

        let (board_tx, board_rx) = mpsc::channel::<BoardProfile>();
        drop(board_rx);

        let ctx_for_thread = Arc::clone(&ctx);
        let server = thread::spawn(move || {
            for _ in 0..3 {
                let (stream, _) = listener.accept().expect("test client should connect");
                handle_connection(stream, Arc::clone(&ctx_for_thread), board_tx.clone());
            }
        });

        let body = r#"{"show_bus_labels":true}"#;
        let post_request = format!(
            "POST /api/wiring HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        let post_response = send_request(addr, &post_request);
        assert!(post_response.contains(r#""show_bus_labels":true"#));

        let wiring_response =
            send_request(addr, "GET /api/wiring HTTP/1.1\r\nHost: localhost\r\n\r\n");
        assert!(wiring_response.contains(r#""show_bus_labels":true"#));

        let svg_response = send_request(
            addr,
            "GET /api/wiring/svg HTTP/1.1\r\nHost: localhost\r\n\r\n",
        );
        assert!(svg_response.contains(r#"class="dev-pin""#));

        server.join().expect("server thread should exit");
    }

    #[test]
    fn device_simulation_rig_only_polls_selected_devices() {
        let mut rig = DeviceSimulationRig::new(BoardProfile::OriginalEsp32);
        let wiring_state = WiringState {
            board: BoardProfile::OriginalEsp32,
            sensor_profile: SensorProfile::Minimal,
            selected_devices: vec![DeviceKind::Bme280, DeviceKind::Lcd1602],
            show_bus_labels: false,
        };

        let state = rig.step(&wiring_state);

        assert_eq!(
            state.wiring.attached_devices,
            vec!["BME280 (0x77)".to_string(), "LCD1602 (0x27)".to_string()]
        );
        assert!(state.i2c.operation_count > 0);
        assert!(state
            .i2c
            .recent_operations
            .iter()
            .all(|line| line.contains("0x77") || line.contains("0x27")));
        assert!(state.climate.temperature_c.is_some());
        assert_eq!(state.climate.physical_lcd_frame[0].len(), 16);
        assert_eq!(state.distance.distance_mm, None);
        assert_eq!(state.imu.accel_mg, [0, 0, 0]);
        assert_eq!(state.light.lux_x100, 0);
        assert_eq!(state.camera.sequence, 0);
        assert_eq!(state.gas.co2_ppm, None);
        assert_eq!(state.rtc.datetime_str, "");
        assert_eq!(state.tof.distance_mm, None);
    }

    #[test]
    fn device_simulation_rig_keeps_bme280_data_when_lcd_is_disabled() {
        let mut rig = DeviceSimulationRig::new(BoardProfile::OriginalEsp32);
        let wiring_state = WiringState {
            board: BoardProfile::OriginalEsp32,
            sensor_profile: SensorProfile::Minimal,
            selected_devices: vec![DeviceKind::Bme280],
            show_bus_labels: false,
        };

        let state = rig.step(&wiring_state);

        assert_eq!(
            state.wiring.attached_devices,
            vec!["BME280 (0x77)".to_string()]
        );
        assert!(state.climate.temperature_c.is_some());
        assert!(state.climate.humidity_percent.is_some());
        assert!(state.climate.pressure_pa.is_some());
        assert_eq!(state.climate.app_frame, blank_lines());
        assert_eq!(state.climate.physical_lcd_frame, blank_lines());
    }

    #[test]
    fn device_simulation_rig_does_not_fabricate_climate_without_bme280() {
        let mut rig = DeviceSimulationRig::new(BoardProfile::OriginalEsp32);
        let wiring_state = WiringState {
            board: BoardProfile::OriginalEsp32,
            sensor_profile: SensorProfile::Minimal,
            selected_devices: vec![DeviceKind::Lcd1602],
            show_bus_labels: false,
        };

        let state = rig.step(&wiring_state);

        assert_eq!(
            state.wiring.attached_devices,
            vec!["LCD1602 (0x27)".to_string()]
        );
        assert_eq!(state.climate.temperature_c, None);
        assert_eq!(state.climate.humidity_percent, None);
        assert_eq!(state.climate.pressure_pa, None);
        assert_eq!(state.climate.physical_lcd_frame, blank_lines());
    }

    #[test]
    fn device_simulation_rig_reports_consistent_ds3231_address() {
        let mut rig = DeviceSimulationRig::new(BoardProfile::OriginalEsp32);
        let wiring_state = WiringState {
            board: BoardProfile::OriginalEsp32,
            sensor_profile: SensorProfile::ClimateStation,
            selected_devices: vec![DeviceKind::Ds3231],
            show_bus_labels: false,
        };

        let state = rig.step(&wiring_state);

        assert_eq!(
            state.wiring.attached_devices,
            vec!["DS3231 (0x68)".to_string()]
        );
        assert!(
            state
                .i2c
                .recent_operations
                .iter()
                .any(|line| line.contains("0x68")),
            "dashboard should render DS3231 traffic with the logical hardware address"
        );
        assert!(
            state
                .i2c
                .recent_operations
                .iter()
                .all(|line| !line.contains("0x69")),
            "dashboard should not expose the colliding hardware address"
        );
        assert!(!state.rtc.datetime_str.is_empty());
    }

    #[test]
    fn device_simulation_rig_resets_disabled_actuators() {
        let mut rig = DeviceSimulationRig::new(BoardProfile::OriginalEsp32);
        let active_wiring_state = WiringState {
            board: BoardProfile::OriginalEsp32,
            sensor_profile: SensorProfile::RobotBase,
            selected_devices: vec![
                DeviceKind::HcSr04,
                DeviceKind::Mpu6050,
                DeviceKind::Servo,
                DeviceKind::L298n,
            ],
            show_bus_labels: false,
        };

        let active_state = rig.step(&active_wiring_state);
        assert_ne!(active_state.servo.angle_degrees, 0);
        assert_eq!(active_state.motor_driver.left.direction, "forward");
        assert_eq!(active_state.motor_driver.left.duty_percent, 42);
        assert_eq!(active_state.motor_driver.right.direction, "forward");
        assert_eq!(active_state.motor_driver.right.duty_percent, 42);

        let disabled_wiring_state = WiringState {
            board: BoardProfile::OriginalEsp32,
            sensor_profile: SensorProfile::ClimateStation,
            selected_devices: vec![DeviceKind::Ds3231],
            show_bus_labels: false,
        };

        let disabled_state = rig.step(&disabled_wiring_state);
        assert_eq!(disabled_state.servo.angle_degrees, 0);
        assert_eq!(disabled_state.motor_driver.left.direction, "coast");
        assert_eq!(disabled_state.motor_driver.left.duty_percent, 0);
        assert_eq!(disabled_state.motor_driver.right.direction, "coast");
        assert_eq!(disabled_state.motor_driver.right.duty_percent, 0);
    }

    #[test]
    fn wiring_profiles_endpoint_lists_all_profiles() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("listener should bind");
        let addr = listener
            .local_addr()
            .expect("listener should have local addr");
        let ctx = ServerContext::new(BoardProfile::OriginalEsp32);

        let (board_tx, board_rx) = mpsc::channel::<BoardProfile>();
        drop(board_rx);

        let ctx_for_thread = Arc::clone(&ctx);
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("test client should connect");
            handle_connection(stream, ctx_for_thread, board_tx);
        });

        let response = send_request(
            addr,
            "GET /api/wiring/profiles HTTP/1.1\r\nHost: localhost\r\n\r\n",
        );
        assert!(response.contains("\"profiles\":["));
        assert!(response.contains("\"slug\":\"full\""));
        assert!(response.contains("\"slug\":\"climate\""));
        assert!(response.contains("\"slug\":\"robot\""));
        assert!(response.contains("\"slug\":\"minimal\""));
        assert!(response.contains(r#""devices":["bme280","lcd1602"]"#));

        server.join().expect("server thread should exit");
    }

    #[test]
    fn wiring_state_and_svg_reflect_explicit_selection_over_profile() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("listener should bind");
        let addr = listener
            .local_addr()
            .expect("listener should have local addr");
        let ctx = ServerContext::new(BoardProfile::OriginalEsp32);

        let (board_tx, board_rx) = mpsc::channel::<BoardProfile>();
        drop(board_rx);

        let ctx_for_thread = Arc::clone(&ctx);
        let server = thread::spawn(move || {
            for _ in 0..3 {
                let (stream, _) = listener.accept().expect("test client should connect");
                handle_connection(stream, Arc::clone(&ctx_for_thread), board_tx.clone());
            }
        });

        let body = r#"{"sensor_profile":"robot","selected_devices":["bme280","servo"]}"#;
        let post_request = format!(
            "POST /api/wiring HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        let post_response = send_request(addr, &post_request);
        assert!(post_response.contains("\"sensor_profile\":\"robot\""));
        assert!(post_response.contains("\"selected_devices\":[\"bme280\",\"servo\"]"));
        assert!(post_response.contains("\"devices\":["));
        assert!(post_response.contains("\"kind\":\"bme280\""));
        assert!(post_response.contains("\"kind\":\"servo\""));

        let wiring_response =
            send_request(addr, "GET /api/wiring HTTP/1.1\r\nHost: localhost\r\n\r\n");
        assert!(wiring_response.contains("\"selected_devices\":[\"bme280\",\"servo\"]"));
        assert!(wiring_response.contains("\"devices\":["));
        assert!(wiring_response.contains("\"kind\":\"bme280\""));
        assert!(wiring_response.contains("\"kind\":\"servo\""));

        let svg_response = send_request(
            addr,
            "GET /api/wiring/svg HTTP/1.1\r\nHost: localhost\r\n\r\n",
        );
        assert!(svg_response.contains("BME280"));
        assert!(svg_response.contains("Servo"));
        assert!(!svg_response.contains("MPU6050"));

        server.join().expect("server thread should exit");
    }

    #[test]
    fn wiring_endpoint_filters_unsupported_camera_from_arduino_nano() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("listener should bind");
        let addr = listener
            .local_addr()
            .expect("listener should have local addr");
        let ctx = ServerContext::new(BoardProfile::OriginalEsp32);

        let (board_tx, board_rx) = mpsc::channel::<BoardProfile>();
        drop(board_rx);

        let ctx_for_thread = Arc::clone(&ctx);
        let server = thread::spawn(move || {
            for _ in 0..3 {
                let (stream, _) = listener.accept().expect("test client should connect");
                handle_connection(stream, Arc::clone(&ctx_for_thread), board_tx.clone());
            }
        });

        let body = r#"{"board":"arduino-nano","sensor_profile":"full"}"#;
        let post_request = format!(
            "POST /api/wiring HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        let post_response = send_request(addr, &post_request);
        assert!(post_response.contains("\"board\":\"nano\""));
        assert!(!post_response.contains("\"esp32_cam\""));
        assert!(!post_response.contains("ESP32-CAM"));

        let wiring_response =
            send_request(addr, "GET /api/wiring HTTP/1.1\r\nHost: localhost\r\n\r\n");
        assert!(!wiring_response.contains("\"esp32_cam\""));

        let svg_response = send_request(
            addr,
            "GET /api/wiring/svg HTTP/1.1\r\nHost: localhost\r\n\r\n",
        );
        assert!(!svg_response.contains("CAM/N/A"));
        assert!(!svg_response.contains("GPIO:N/A"));

        server.join().expect("server thread should exit");
    }

    // ── board_kind_from_str ───────────────────────────────────────────────────
    #[test]
    fn board_kind_from_str_known_values() {
        assert!(matches!(board_kind_from_str("esp32"), BoardKind::Esp32));
        assert!(matches!(
            board_kind_from_str("m5stickc"),
            BoardKind::M5StickC
        ));
        assert!(matches!(
            board_kind_from_str("arduino-nano"),
            BoardKind::ArduinoNano
        ));
        assert!(matches!(
            board_kind_from_str("raspi-pico"),
            BoardKind::RaspberryPiPico
        ));
    }

    #[test]
    fn board_kind_from_str_defaults_to_esp32() {
        assert!(matches!(board_kind_from_str(""), BoardKind::Esp32));
        assert!(matches!(board_kind_from_str("unknown"), BoardKind::Esp32));
    }

    // ── detect_build_target / detect_binary_name ─────────────────────────────
    #[test]
    fn detect_build_target_reads_config_toml() {
        let dir = tempfile::tempdir().expect("tmp dir");
        let cargo_dir = dir.path().join(".cargo");
        std::fs::create_dir_all(&cargo_dir).expect("create .cargo");
        std::fs::write(
            cargo_dir.join("config.toml"),
            "[build]\ntarget = \"xtensa-esp32-none-elf\"\n",
        )
        .expect("write config.toml");
        assert_eq!(
            detect_build_target(dir.path()),
            Some("xtensa-esp32-none-elf".to_string())
        );
    }

    #[test]
    fn detect_build_target_returns_none_if_no_config() {
        let dir = tempfile::tempdir().expect("tmp dir");
        assert_eq!(detect_build_target(dir.path()), None);
    }

    #[test]
    fn detect_binary_name_reads_cargo_toml() {
        let dir = tempfile::tempdir().expect("tmp dir");
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"my-firmware\"\nversion = \"0.1.0\"\n",
        )
        .expect("write Cargo.toml");
        assert_eq!(
            detect_binary_name(dir.path()),
            Some("my-firmware".to_string())
        );
    }

    #[test]
    fn detect_binary_name_returns_none_if_no_cargo_toml() {
        let dir = tempfile::tempdir().expect("tmp dir");
        assert_eq!(detect_binary_name(dir.path()), None);
    }

    // ── SensorHistoryBuffer ───────────────────────────────────────────────────

    #[test]
    fn sensor_history_buffer_caps_at_capacity() {
        let mut buf = SensorHistoryBuffer::new(3);
        for i in 0..5u32 {
            buf.push_climate(i as i32 * 100, i * 10, Some(101000 + i));
        }
        assert_eq!(buf.climate.len(), 3);
        // Oldest entries should be evicted — last 3 pushed are indices 2..4
        assert_eq!(buf.climate[0].0, 200);
        assert_eq!(buf.climate[2].0, 400);
    }

    #[test]
    fn sensor_history_buffer_distance_caps_at_capacity() {
        let mut buf = SensorHistoryBuffer::new(5);
        for i in 0..10u32 {
            buf.push_distance(Some(i * 10));
        }
        assert_eq!(buf.distance.len(), 5);
    }

    #[test]
    fn sensor_history_buffer_climate_json_valid() {
        let mut buf = SensorHistoryBuffer::new(10);
        buf.push_climate(2500, 6000, Some(101325));
        buf.push_climate(2600, 5500, None);
        let json = buf.climate_json();
        assert!(json.contains("\"temperature\""));
        assert!(json.contains("\"humidity\""));
        assert!(json.contains("\"pressure\""));
        assert!(json.contains("25.00"));
        assert!(json.contains("null"));
    }

    #[test]
    fn sensor_history_buffer_distance_json_valid() {
        let mut buf = SensorHistoryBuffer::new(10);
        buf.push_distance(Some(350));
        buf.push_distance(None);
        let json = buf.distance_json();
        assert!(json.contains("\"distance\""));
        assert!(json.contains("350"));
        assert!(json.contains("null"));
    }

    #[test]
    fn api_history_endpoint_returns_json() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("listener should bind");
        let addr = listener.local_addr().expect("addr");
        let ctx = ServerContext::new(BoardProfile::OriginalEsp32);
        {
            let mut hist = ctx.history.lock().unwrap();
            hist.push_climate(2500, 6000, Some(101325));
        }

        let (board_tx, board_rx) = mpsc::channel::<BoardProfile>();
        drop(board_rx);
        let ctx_for_thread = Arc::clone(&ctx);
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("test client should connect");
            handle_connection(stream, ctx_for_thread, board_tx);
        });

        let resp = send_request(
            addr,
            "GET /api/history?sensor=bme280 HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        );
        assert!(resp.contains("200 OK"), "expected 200, got: {resp}");
        assert!(
            resp.contains("\"temperature\""),
            "body missing temperature: {resp}"
        );
        assert!(
            resp.contains("\"humidity\""),
            "body missing humidity: {resp}"
        );
        assert!(resp.contains("25.00"), "body missing value: {resp}");
        server.join().expect("server thread should exit");
    }

    #[test]
    fn api_history_endpoint_distance_returns_json() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("listener should bind");
        let addr = listener.local_addr().expect("addr");
        let ctx = ServerContext::new(BoardProfile::OriginalEsp32);
        {
            let mut hist = ctx.history.lock().unwrap();
            hist.push_distance(Some(400));
        }

        let (board_tx, board_rx) = mpsc::channel::<BoardProfile>();
        drop(board_rx);
        let ctx_for_thread = Arc::clone(&ctx);
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("test client should connect");
            handle_connection(stream, ctx_for_thread, board_tx);
        });

        let resp = send_request(
            addr,
            "GET /api/history?sensor=distance HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        );
        assert!(resp.contains("200 OK"), "expected 200, got: {resp}");
        assert!(
            resp.contains("\"distance\""),
            "body missing distance: {resp}"
        );
        assert!(resp.contains("400"), "body missing value: {resp}");
        server.join().expect("server thread should exit");
    }
}
