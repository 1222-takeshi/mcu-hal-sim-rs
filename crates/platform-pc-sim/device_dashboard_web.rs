use core::str;
use std::env;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};

use core_app::climate_display::{ClimateDisplayApp, ClimateDisplayConfig};
use embedded_hal::delay::DelayNs;
use hal_api::actuator::{DualMotorDriver, MotorCommand, MotorDirection, ServoMotor};
use hal_api::distance::DistanceSensor;
use hal_api::imu::ImuSensor;
use platform_pc_sim::bme280_mock::{demo_raw_samples, MockBme280Device};
use platform_pc_sim::dashboard::BoardProfile;
use platform_pc_sim::hc_sr04_mock::{demo_echo_pulses_us, MockHcSr04Device};
use platform_pc_sim::lcd1602_mock::MockLcd1602Device;
use platform_pc_sim::mock_hal::MockPin;
use platform_pc_sim::mpu6050_mock::{demo_raw_frames, MockMpu6050Device};
use platform_pc_sim::pwm_mock::MockPwmOutput;
use platform_pc_sim::virtual_i2c::{VirtualI2cBus, VirtualI2cOperation};
use platform_pc_sim::web_dashboard::{
    dashboard_html, state_to_json, ClimatePanelState, DeviceDashboardState, DistancePanelState,
    I2cPanelState, ImuPanelState, MotorChannelState, MotorDriverPanelState, ServoPanelState,
    WiringPanelState,
};
use platform_pc_sim::wiring_config::WiringConfig;
use platform_pc_sim::wiring_svg::wiring_svg;
use reference_drivers::bme280::{Bme280Sensor, BME280_ADDRESS_PRIMARY};
use reference_drivers::hc_sr04::HcSr04Sensor;
use reference_drivers::l298n::{L298nChannel, L298nDualDriver};
use reference_drivers::lcd1602::{Lcd1602Display, LCD1602_ADDRESS_PRIMARY};
use reference_drivers::mpu6050::{Mpu6050Sensor, MPU6050_ADDRESS_PRIMARY};
use reference_drivers::servo::ServoDriver;

const DEFAULT_PORT: u16 = 7878;

#[derive(Default)]
struct NoopDelay;

impl DelayNs for NoopDelay {
    fn delay_ns(&mut self, _ns: u32) {}
    fn delay_us(&mut self, _us: u32) {}
    fn delay_ms(&mut self, _ms: u32) {}
}

type ServoRig = ServoDriver<MockPwmOutput>;
type MotorChannelRig = L298nChannel<MockPin, MockPin, MockPwmOutput>;
type MotorDriverRig = L298nDualDriver<MotorChannelRig, MotorChannelRig>;

struct DeviceSimulationRig {
    board: BoardProfile,
    bus: VirtualI2cBus,
    bme280: MockBme280Device,
    lcd: MockLcd1602Device,
    mpu6050: MockMpu6050Device,
    app: ClimateDisplayApp<Bme280Sensor<VirtualI2cBus>, Lcd1602Display<VirtualI2cBus, NoopDelay>>,
    bme280_samples: Vec<[u8; 8]>,
    bme280_sample_index: usize,
    distance_sensor: HcSr04Sensor<MockHcSr04Device>,
    imu_sensor: Mpu6050Sensor<VirtualI2cBus>,
    imu_frames: Vec<[u8; 14]>,
    imu_frame_index: usize,
    servo: ServoRig,
    motor_driver: MotorDriverRig,
    last_distance_mm: Option<u32>,
    last_imu: Option<hal_api::imu::ImuReading>,
}

impl DeviceSimulationRig {
    fn new(board: BoardProfile) -> Self {
        let bus = VirtualI2cBus::new();
        let bme280 = MockBme280Device::new();
        let lcd = MockLcd1602Device::new();
        let mpu6050 = MockMpu6050Device::new();
        bus.attach_device(BME280_ADDRESS_PRIMARY, bme280.clone());
        bus.attach_device(LCD1602_ADDRESS_PRIMARY, lcd.clone());
        bus.attach_device(MPU6050_ADDRESS_PRIMARY, mpu6050.clone());

        let app = ClimateDisplayApp::new_with_config(
            Bme280Sensor::new(bus.clone()),
            Lcd1602Display::new(bus.clone(), NoopDelay),
            ClimateDisplayConfig {
                refresh_period_ticks: 5,
                refresh_on_first_tick: true,
            },
        );
        let distance_sensor = HcSr04Sensor::new(MockHcSr04Device::looping(demo_echo_pulses_us()));
        let imu_sensor = Mpu6050Sensor::new(bus.clone());

        let servo = ServoDriver::new(MockPwmOutput::new());
        let motor_driver = L298nDualDriver::new(
            L298nChannel::new(MockPin::new(0), MockPin::new(0), MockPwmOutput::new()),
            L298nChannel::new(MockPin::new(0), MockPin::new(0), MockPwmOutput::new()),
        );

        Self {
            board,
            bus,
            bme280,
            lcd,
            mpu6050,
            app,
            bme280_samples: demo_raw_samples(),
            bme280_sample_index: 0,
            distance_sensor,
            imu_sensor,
            imu_frames: demo_raw_frames(),
            imu_frame_index: 0,
            servo,
            motor_driver,
            last_distance_mm: None,
            last_imu: None,
        }
    }

    fn step(&mut self) -> DeviceDashboardState {
        self.bme280
            .set_raw_sample(self.bme280_samples[self.bme280_sample_index]);
        self.bme280_sample_index = (self.bme280_sample_index + 1) % self.bme280_samples.len();
        self.mpu6050
            .set_raw_frame(self.imu_frames[self.imu_frame_index]);
        self.imu_frame_index = (self.imu_frame_index + 1) % self.imu_frames.len();

        self.app
            .tick()
            .expect("dashboard climate app should keep running");
        let tick = self.app.tick_count();

        if tick == 1 || tick % 2 == 0 {
            self.last_distance_mm = Some(
                self.distance_sensor
                    .read_distance()
                    .expect("distance driver should read from host-side pulse device")
                    .distance_mm,
            );
        }

        if tick == 1 || tick % 3 == 0 {
            self.last_imu = Some(
                self.imu_sensor
                    .read_imu()
                    .expect("imu driver should read from host-side mock device"),
            );
        }

        let servo_angle = distance_to_servo_angle(self.last_distance_mm.unwrap_or(180));
        self.servo
            .set_angle_degrees(servo_angle)
            .expect("servo angle should remain in range");

        let (left, right) = motor_commands_from_state(self.last_distance_mm, self.last_imu);
        self.motor_driver
            .apply_channels(left, right)
            .expect("motor commands should remain in range");

        let attached_devices = self
            .bus
            .attached_addresses()
            .into_iter()
            .map(|addr| format!("0x{addr:02X}"))
            .collect::<Vec<_>>();
        let recent_operations = self
            .bus
            .operations()
            .iter()
            .rev()
            .take(10)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(format_operation)
            .collect::<Vec<_>>();
        let app_frame = self
            .app
            .last_frame()
            .map(frame_to_lines)
            .unwrap_or_else(blank_lines);
        let physical_lcd_frame = frame_to_lines(self.lcd.frame());
        let climate = self.app.last_reading();
        let imu = self
            .last_imu
            .unwrap_or_else(|| hal_api::imu::ImuReading::new([0, 0, 0], [0, 0, 0], None));

        DeviceDashboardState {
            board_name: self.board.name().to_string(),
            mcu_name: self.board.mcu().to_string(),
            tick,
            climate: ClimatePanelState {
                temperature_c: climate.map(|value| value.temperature_centi_celsius as f32 / 100.0),
                humidity_percent: climate.map(|value| value.humidity_centi_percent as f32 / 100.0),
                pressure_pa: climate.and_then(|value| value.pressure_pascal),
                app_frame,
                physical_lcd_frame,
            },
            distance: DistancePanelState {
                distance_mm: self.last_distance_mm,
                sensor_name: "HC-SR04".to_string(),
            },
            imu: ImuPanelState {
                sensor_name: "MPU6050".to_string(),
                accel_mg: imu.accel_mg,
                gyro_mdps: imu.gyro_mdps,
                temperature_c: imu
                    .temperature_centi_celsius
                    .map(|value| value as f32 / 100.0),
            },
            servo: ServoPanelState {
                angle_degrees: self.servo.current_angle(),
            },
            motor_driver: MotorDriverPanelState {
                driver_name: "L298N dual H-bridge".to_string(),
                left: channel_state(self.motor_driver.channel_a().current_command()),
                right: channel_state(self.motor_driver.channel_b().current_command()),
            },
            wiring: WiringPanelState {
                sda_pin: self.board.sda_pin().to_string(),
                scl_pin: self.board.scl_pin().to_string(),
                power_pin: self.board.power_pin().to_string(),
                ground_pin: "GND".to_string(),
                diagram_lines: build_wiring_diagram(self.board, &attached_devices),
                attached_devices,
            },
            i2c: I2cPanelState {
                operation_count: self.bus.operation_count(),
                recent_operations,
            },
        }
    }
}

fn blank_lines() -> [String; 2] {
    [
        String::from("                "),
        String::from("                "),
    ]
}

fn frame_to_lines(frame: hal_api::display::TextFrame16x2) -> [String; 2] {
    [line_to_string(&frame, 0), line_to_string(&frame, 1)]
}

fn line_to_string(frame: &hal_api::display::TextFrame16x2, row: usize) -> String {
    str::from_utf8(frame.line(row))
        .unwrap_or("????????????????")
        .to_string()
}

fn addr_to_device_name(addr: &str) -> &'static str {
    match addr {
        "0x27" => "LCD1602",
        "0x68" => "MPU6050",
        "0x77" => "BME280",
        _ => "unknown",
    }
}

fn build_wiring_diagram(board: BoardProfile, attached_devices: &[String]) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let sda_prefix = format!("{} SDA ", board.sda_pin());
    let cont = " ".repeat(sda_prefix.len());

    lines.push("── I2C Bus ──────────────────────────────────────".to_string());
    if attached_devices.is_empty() {
        lines.push(format!("{}---- (no devices)", sda_prefix));
    } else {
        for (i, addr) in attached_devices.iter().enumerate() {
            let name = addr_to_device_name(addr);
            if i == 0 {
                lines.push(format!("{}--+-- {} ({})", sda_prefix, name, addr));
            } else {
                lines.push(format!("{}  +-- {} ({})", cont, name, addr));
            }
        }
    }
    lines.push(format!("{} SCL ---- (shared bus)", board.scl_pin()));
    lines.push(format!("{} VCC ---- sensor power", board.power_pin()));
    lines.push("GND      ---- shared ground".to_string());
    lines.push(String::new());

    lines.push("── GPIO ─────────────────────────────────────────".to_string());
    lines.push(format!("{} TRIG --- HC-SR04 TRIG", board.trig_pin()));
    lines.push(format!("{} ECHO --- HC-SR04 ECHO", board.echo_pin()));
    lines.push(format!("{} PWM  --- Servo signal", board.servo_pwm_pin()));
    lines.push(String::new());

    lines.push("── Motor Driver (L298N-style) ───────────────────".to_string());
    lines.push(format!(
        "{} ENA  --- Motor-A enable (PWM)",
        board.motor_ena_pin()
    ));
    lines.push(format!(
        "{} IN1  --- Motor-A direction 1",
        board.motor_in1_pin()
    ));
    lines.push(format!(
        "{} IN2  --- Motor-A direction 2",
        board.motor_in2_pin()
    ));
    lines.push(format!(
        "{} ENB  --- Motor-B enable (PWM)",
        board.motor_enb_pin()
    ));
    lines.push(format!(
        "{} IN3  --- Motor-B direction 1",
        board.motor_in3_pin()
    ));
    lines.push(format!(
        "{} IN4  --- Motor-B direction 2",
        board.motor_in4_pin()
    ));

    lines
}

fn distance_to_servo_angle(distance_mm: u32) -> u16 {
    let clamped = distance_mm.clamp(80, 360) - 80;
    ((clamped * 180) / 280) as u16
}

fn motor_commands_from_state(
    distance_mm: Option<u32>,
    imu: Option<hal_api::imu::ImuReading>,
) -> (MotorCommand, MotorCommand) {
    let distance_mm = distance_mm.unwrap_or(180);
    let imu = imu.unwrap_or_else(|| hal_api::imu::ImuReading::new([0, 0, 0], [0, 0, 0], None));

    if distance_mm < 160 {
        return (
            MotorCommand::new(MotorDirection::Reverse, 30),
            MotorCommand::new(MotorDirection::Reverse, 30),
        );
    }

    let yaw = imu.gyro_mdps[2];
    if yaw > 40 {
        (
            MotorCommand::new(MotorDirection::Forward, 28),
            MotorCommand::new(MotorDirection::Forward, 46),
        )
    } else if yaw < -40 {
        (
            MotorCommand::new(MotorDirection::Forward, 46),
            MotorCommand::new(MotorDirection::Forward, 28),
        )
    } else {
        (
            MotorCommand::new(MotorDirection::Forward, 42),
            MotorCommand::new(MotorDirection::Forward, 42),
        )
    }
}

fn channel_state(command: MotorCommand) -> MotorChannelState {
    MotorChannelState {
        direction: match command.direction {
            MotorDirection::Forward => "forward",
            MotorDirection::Reverse => "reverse",
            MotorDirection::Brake => "brake",
            MotorDirection::Coast => "coast",
        }
        .to_string(),
        duty_percent: command.duty_percent,
    }
}

fn format_operation(operation: &VirtualI2cOperation) -> String {
    match operation {
        VirtualI2cOperation::Write { addr, bytes } => {
            format!("WRITE addr=0x{addr:02X} bytes={bytes:02X?}")
        }
        VirtualI2cOperation::Read { addr, len } => {
            format!("READ addr=0x{addr:02X} len={len}")
        }
        VirtualI2cOperation::WriteRead { addr, bytes, len } => {
            format!("WRITE_READ addr=0x{addr:02X} bytes={bytes:02X?} len={len}")
        }
    }
}

/// Extract the `"board"` string value from a minimal JSON object.
///
/// Handles `{"board":"arduino-nano"}` without pulling in a full JSON parser.
fn parse_board_from_json(json: &str) -> Option<&str> {
    let after_key = json.split("\"board\"").nth(1)?;
    let after_colon = after_key.split(':').nth(1)?.trim_start();
    let inner = after_colon.strip_prefix('"')?;
    let end = inner.find('"')?;
    Some(&inner[..end])
}

fn respond(stream: &mut TcpStream, status: &str, content_type: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());
}

/// Stream `cargo test --workspace` output as Server-Sent Events.
///
/// Blocks until the test process exits. Each stdout/stderr line is sent as
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
    let mut rig = DeviceSimulationRig::new(board);

    println!("device dashboard server started");
    println!("open http://127.0.0.1:{port}");
    println!("board profile: {}", board.name());

    for stream in listener.incoming() {
        let Ok(mut stream) = stream else {
            continue;
        };
        let mut request = [0u8; 1024];
        let Ok(read_len) = stream.read(&mut request) else {
            continue;
        };
        let request = String::from_utf8_lossy(&request[..read_len]);
        let first_line = request.lines().next().unwrap_or("GET / HTTP/1.1");
        let mut parts = first_line.split_whitespace();
        let method = parts.next().unwrap_or("GET");
        let path = parts.next().unwrap_or("/");
        let body = request
            .find("\r\n\r\n")
            .map(|pos| &request[pos + 4..])
            .unwrap_or("");

        match (method, path) {
            (_, "/") => respond(
                &mut stream,
                "200 OK",
                "text/html; charset=utf-8",
                dashboard_html(),
            ),
            (_, "/api/state") => {
                let payload = state_to_json(&rig.step());
                respond(
                    &mut stream,
                    "200 OK",
                    "application/json; charset=utf-8",
                    &payload,
                );
            }
            ("POST", "/api/wiring") => {
                if let Some(board_name) = parse_board_from_json(body) {
                    let new_board = BoardProfile::from_arg(Some(board_name));
                    rig = DeviceSimulationRig::new(new_board);
                    println!("board changed to: {}", rig.board.name());
                }
                let payload = WiringConfig::from_board(rig.board).to_json();
                respond(
                    &mut stream,
                    "200 OK",
                    "application/json; charset=utf-8",
                    &payload,
                );
            }
            (_, "/api/wiring") => {
                let payload = WiringConfig::from_board(rig.board).to_json();
                respond(
                    &mut stream,
                    "200 OK",
                    "application/json; charset=utf-8",
                    &payload,
                );
            }
            (_, "/api/wiring/svg") => {
                let cfg = WiringConfig::from_board(rig.board);
                let svg = wiring_svg(&cfg);
                respond(&mut stream, "200 OK", "image/svg+xml; charset=utf-8", &svg);
            }
            (_, "/api/test/stream") => {
                handle_test_stream(&mut stream);
            }
            _ => respond(
                &mut stream,
                "404 Not Found",
                "text/plain; charset=utf-8",
                "not found",
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
