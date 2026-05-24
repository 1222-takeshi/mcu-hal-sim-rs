//! Browser dashboard state and HTML renderer.

use std::fmt::Write as _;
use std::string::String;

#[derive(Debug, Clone)]
pub struct DeviceDashboardState {
    pub board_name: String,
    pub mcu_name: String,
    pub tick: u32,
    pub climate: ClimatePanelState,
    pub distance: DistancePanelState,
    pub imu: ImuPanelState,
    pub servo: ServoPanelState,
    pub motor_driver: MotorDriverPanelState,
    pub wiring: WiringPanelState,
    pub i2c: I2cPanelState,
    pub light: LightPanelState,
    pub camera: CameraPanelState,
    pub gas: GasPanelState,
    pub rtc: RtcPanelState,
    pub tof: TofPanelState,
    pub diagnostics: DiagnosticsPanelState,
}

#[derive(Debug, Clone)]
pub struct ClimatePanelState {
    pub temperature_c: Option<f32>,
    pub humidity_percent: Option<f32>,
    pub pressure_pa: Option<u32>,
    pub app_frame: [String; 2],
    pub physical_lcd_frame: [String; 2],
}

#[derive(Debug, Clone)]
pub struct DistancePanelState {
    pub distance_mm: Option<u32>,
    pub sensor_name: String,
}

#[derive(Debug, Clone)]
pub struct ImuPanelState {
    pub sensor_name: String,
    pub accel_mg: [i16; 3],
    pub gyro_mdps: [i32; 3],
    pub temperature_c: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct ServoPanelState {
    pub angle_degrees: u16,
}

#[derive(Debug, Clone)]
pub struct MotorChannelState {
    pub direction: String,
    pub duty_percent: u8,
}

#[derive(Debug, Clone)]
pub struct MotorDriverPanelState {
    pub driver_name: String,
    pub left: MotorChannelState,
    pub right: MotorChannelState,
}

#[derive(Debug, Clone)]
pub struct WiringPanelState {
    pub sda_pin: String,
    pub scl_pin: String,
    pub power_pin: String,
    pub ground_pin: String,
    pub attached_devices: Vec<String>,
    pub selected_devices: Vec<String>,
    pub show_bus_labels: bool,
    pub diagram_lines: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct I2cPanelState {
    pub operation_count: usize,
    pub recent_operations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct LightPanelState {
    pub lux_x100: u32,
    pub sensor_name: String,
}

#[derive(Debug, Clone)]
pub struct CameraPanelState {
    pub width: u32,
    pub height: u32,
    pub sequence: u32,
    pub sensor_name: String,
}

#[derive(Debug, Clone)]
pub struct GasPanelState {
    pub co2_ppm: Option<u16>,
    pub voc_ppb: Option<u16>,
    pub sensor_name: String,
}

#[derive(Debug, Clone)]
pub struct RtcPanelState {
    pub datetime_str: String,
    pub sensor_name: String,
}

#[derive(Debug, Clone)]
pub struct TofPanelState {
    pub distance_mm: Option<u32>,
    pub sensor_name: String,
}

/// Diagnostics ring buffer state surfaced per tick.
///
/// A single diagnostics event with elapsed time, severity, and message.
#[derive(Debug, Clone, Default)]
pub struct DiagEvent {
    /// Milliseconds elapsed since simulator start.
    pub elapsed_ms: u64,
    /// Severity level: "info", "warn", or "error".
    pub severity: String,
    /// Human-readable event description.
    pub message: String,
}

/// `recent_events` holds up to 20 entries most-recent-first.
/// `event_count` is a monotonically increasing counter of all events ever
/// logged (useful for detecting new activity without diffing the list).
#[derive(Debug, Clone, Default)]
pub struct DiagnosticsPanelState {
    pub recent_events: Vec<DiagEvent>,
    pub event_count: u32,
}

pub fn state_to_json(state: &DeviceDashboardState) -> String {
    let mut output = String::new();
    let _ = write!(
        output,
        "{{\"board_name\":{},\"mcu_name\":{},\"tick\":{},\"climate\":{{\"temperature_c\":{},\"humidity_percent\":{},\"pressure_pa\":{},\"app_frame\":[{},{}],\"physical_lcd_frame\":[{},{}]}},\"distance\":{{\"distance_mm\":{},\"sensor_name\":{}}},\"imu\":{{\"sensor_name\":{},\"accel_mg\":[{},{},{}],\"gyro_mdps\":[{},{},{}],\"temperature_c\":{}}},\"servo\":{{\"angle_degrees\":{}}},\"motor_driver\":{{\"driver_name\":{},\"left\":{{\"direction\":{},\"duty_percent\":{}}},\"right\":{{\"direction\":{},\"duty_percent\":{}}}}},\"wiring\":{{\"sda_pin\":{},\"scl_pin\":{},\"power_pin\":{},\"ground_pin\":{},\"attached_devices\":[{}],\"selected_devices\":[{}],\"show_bus_labels\":{},\"diagram_lines\":[{}]}},\"i2c\":{{\"operation_count\":{},\"recent_operations\":[{}]}},\"light\":{{\"lux_x100\":{},\"lux\":{},\"sensor_name\":{}}},\"camera\":{{\"width\":{},\"height\":{},\"sequence\":{},\"sensor_name\":{}}},\"gas\":{{\"co2_ppm\":{},\"voc_ppb\":{},\"sensor_name\":{}}},\"rtc\":{{\"datetime_str\":{},\"sensor_name\":{}}},\"tof\":{{\"distance_mm\":{},\"sensor_name\":{}}},\"diagnostics\":{{\"event_count\":{},\"recent_events\":[{}]}}}}",
        json_string(&state.board_name),
        json_string(&state.mcu_name),
        state.tick,
        json_option_f32(state.climate.temperature_c),
        json_option_f32(state.climate.humidity_percent),
        json_option_u32(state.climate.pressure_pa),
        json_string(&state.climate.app_frame[0]),
        json_string(&state.climate.app_frame[1]),
        json_string(&state.climate.physical_lcd_frame[0]),
        json_string(&state.climate.physical_lcd_frame[1]),
        json_option_u32(state.distance.distance_mm),
        json_string(&state.distance.sensor_name),
        json_string(&state.imu.sensor_name),
        state.imu.accel_mg[0],
        state.imu.accel_mg[1],
        state.imu.accel_mg[2],
        state.imu.gyro_mdps[0],
        state.imu.gyro_mdps[1],
        state.imu.gyro_mdps[2],
        json_option_f32(state.imu.temperature_c),
        state.servo.angle_degrees,
        json_string(&state.motor_driver.driver_name),
        json_string(&state.motor_driver.left.direction),
        state.motor_driver.left.duty_percent,
        json_string(&state.motor_driver.right.direction),
        state.motor_driver.right.duty_percent,
        json_string(&state.wiring.sda_pin),
        json_string(&state.wiring.scl_pin),
        json_string(&state.wiring.power_pin),
        json_string(&state.wiring.ground_pin),
        join_json_strings(&state.wiring.attached_devices),
        join_json_strings(&state.wiring.selected_devices),
        state.wiring.show_bus_labels,
        join_json_strings(&state.wiring.diagram_lines),
        state.i2c.operation_count,
        join_json_strings(&state.i2c.recent_operations),
        state.light.lux_x100,
        state.light.lux_x100 / 100,
        json_string(&state.light.sensor_name),
        state.camera.width,
        state.camera.height,
        state.camera.sequence,
        json_string(&state.camera.sensor_name),
        json_option_u16(state.gas.co2_ppm),
        json_option_u16(state.gas.voc_ppb),
        json_string(&state.gas.sensor_name),
        json_string(&state.rtc.datetime_str),
        json_string(&state.rtc.sensor_name),
        json_option_u32(state.tof.distance_mm),
        json_string(&state.tof.sensor_name),
        state.diagnostics.event_count,
        join_diag_events(&state.diagnostics.recent_events),
    );
    output
}

fn json_option_f32(value: Option<f32>) -> String {
    value
        .map(|value| {
            let mut output = String::new();
            let _ = write!(output, "{value:.2}");
            output
        })
        .unwrap_or_else(|| "null".to_string())
}

fn json_option_u32(value: Option<u32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn json_option_u16(value: Option<u16>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

pub fn join_diag_events(events: &[DiagEvent]) -> String {
    let mut output = String::new();
    for (i, ev) in events.iter().enumerate() {
        if i != 0 {
            output.push(',');
        }
        let _ = write!(
            output,
            "{{\"ts\":{},\"sev\":{},\"msg\":{}}}",
            ev.elapsed_ms,
            json_string(&ev.severity),
            json_string(&ev.message),
        );
    }
    output
}

fn join_json_strings(values: &[String]) -> String {
    let mut output = String::new();
    for (index, value) in values.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str(&json_string(value));
    }
    output
}

fn json_string(value: &str) -> String {
    let mut output = String::from("\"");
    for character in value.chars() {
        match character {
            '\\' => output.push_str("\\\\"),
            '"' => output.push_str("\\\""),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                output.push_str(&format!("\\u{:04X}", c as u32));
            }
            _ => output.push(character),
        }
    }
    output.push('"');
    output
}

pub fn dashboard_html() -> &'static str {
    r##"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Device Dashboard</title>
  <style>
    :root {
      --bg: #f6f1e7;
      --paper: rgba(255,252,247,0.88);
      --ink: #1c2b28;
      --muted: #6c756f;
      --accent: #0f7c6b;
      --accent-2: #d95f3d;
      --line: rgba(28,43,40,0.12);
      --shadow: 0 24px 80px rgba(33,41,38,0.12);
      --status-ok: #0f7c6b;
      --status-err: #d95f3d;
      --spark: #0f7c6b;
      --grad1: rgba(15,124,107,0.18);
      --grad2: rgba(217,95,61,0.12);
      --grad3: #faf5eb;
    }
    .dark {
      --bg: #0e1614;
      --paper: rgba(20,30,28,0.92);
      --ink: #dde8e4;
      --muted: #7a9990;
      --accent: #3dcfb8;
      --accent-2: #f07850;
      --line: rgba(221,232,228,0.1);
      --shadow: 0 24px 80px rgba(0,0,0,0.4);
      --status-ok: #3dcfb8;
      --status-err: #f07850;
      --spark: #3dcfb8;
      --grad1: rgba(61,207,184,0.1);
      --grad2: rgba(240,120,80,0.08);
      --grad3: #0a1210;
    }
    * { box-sizing: border-box; }
    html { transition: color 0.25s, background 0.25s; }
    body {
      margin: 0;
      font-family: "IBM Plex Sans", "Avenir Next", sans-serif;
      color: var(--ink);
      background:
        radial-gradient(circle at top left, var(--grad1), transparent 28rem),
        radial-gradient(circle at top right, var(--grad2), transparent 24rem),
        linear-gradient(180deg, var(--grad3) 0%, var(--bg) 100%);
      min-height: 100vh;
    }
    .shell {
      width: min(1220px, calc(100vw - 32px));
      margin: 0 auto 40px;
    }
    /* ── Status bar ── */
    .status-bar {
      display: flex;
      align-items: center;
      gap: 10px;
      padding: 10px 2px;
      font-size: 13px;
      flex-wrap: wrap;
    }
    .sdot {
      width: 8px; height: 8px;
      border-radius: 50%;
      background: var(--muted);
      flex-shrink: 0;
      transition: background 0.3s, box-shadow 0.3s;
    }
    .sdot.ok  { background: var(--status-ok);  box-shadow: 0 0 6px var(--status-ok); }
    .sdot.err { background: var(--status-err); box-shadow: 0 0 6px var(--status-err); }
    .stext { color: var(--muted); }
    .serr  { color: var(--status-err); font-family: monospace; font-size: 12px;
             max-width: 260px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
    .ctrls { display: flex; align-items: center; gap: 8px; margin-left: auto; }
    select, .btn {
      font-family: inherit; font-size: 13px;
      border: 1px solid var(--line); border-radius: 8px;
      padding: 5px 10px;
      background: var(--paper); color: var(--ink);
      cursor: pointer; transition: background 0.2s;
    }
    select:hover, .btn:hover { background: rgba(15,124,107,0.1); }
    /* ── Hero ── */
    .hero {
      display: grid;
      grid-template-columns: 2fr 1fr;
      gap: 16px;
      margin-bottom: 16px;
    }
    .panel {
      background: var(--paper);
      border: 1px solid var(--line);
      border-radius: 24px;
      box-shadow: var(--shadow);
      backdrop-filter: blur(18px);
    }
    .hero-main { padding: 24px 24px 18px; position: relative; overflow: hidden; }
    .hero-main::after {
      content: ""; position: absolute; inset: auto -12% -40% auto;
      width: 220px; height: 220px;
      background: radial-gradient(circle, rgba(15,124,107,0.18), transparent 65%);
      pointer-events: none;
    }
    .kicker {
      font-size: 12px; letter-spacing: 0.18em;
      text-transform: uppercase; color: var(--accent); margin-bottom: 10px;
    }
    .hero-title {
      font-family: "Iowan Old Style","Palatino Linotype",serif;
      font-size: clamp(34px,5vw,58px); line-height: 0.95; margin: 0 0 10px;
    }
    .hero-sub { color: var(--muted); max-width: 42rem; line-height: 1.5; }
    .hero-meta { display: flex; gap: 10px; flex-wrap: wrap; margin-top: 18px; }
    .chip {
      padding: 8px 12px; border-radius: 999px;
      background: rgba(15,124,107,0.08); color: var(--ink); font-size: 13px;
    }
    .hero-side { padding: 20px; display: grid; align-content: space-between; }
    .hero-side .label { color: var(--muted); font-size: 13px; margin-bottom: 6px; }
    .hero-side .big   { font-family: "Iowan Old Style",serif; font-size: 36px; margin-bottom: 14px; }
    /* ── Grid ── */
    .grid { display: grid; grid-template-columns: repeat(12,1fr); gap: 16px; }
    .card { padding: 18px; min-height: 180px; }
    .span-4  { grid-column: span 4; }
    .span-6  { grid-column: span 6; }
    .span-8  { grid-column: span 8; }
    .span-12 { grid-column: span 12; }
    h2 { margin: 0 0 12px; font-size: 18px; }
    /* ── Metric ── */
    .metric-row { display: grid; grid-template-columns: repeat(3,1fr); gap: 10px; }
    .metric { padding: 12px; border-radius: 16px; background: rgba(28,43,40,0.04); }
    .metric .name {
      color: var(--muted); font-size: 12px; margin-bottom: 4px;
      text-transform: uppercase; letter-spacing: 0.08em;
    }
    .metric .val { font-size: 22px; font-weight: 700; }
    /* ── Sparkline ── */
    .spark-wrap { margin-top: 8px; }
    svg.spark {
      display: block; width: 100%; height: 34px; overflow: visible;
    }
    svg.spark polyline {
      fill: none; stroke: var(--spark); stroke-width: 1.8;
      stroke-linejoin: round; stroke-linecap: round;
    }
    /* ── LCD ── */
    .lcd {
      display: inline-block; padding: 16px 18px; border-radius: 18px;
      background: linear-gradient(180deg,#9bc39f 0%,#84b18a 100%);
      color: #112315; font-family: "IBM Plex Mono","Menlo",monospace;
      font-size: 18px; box-shadow: inset 0 0 0 1px rgba(17,35,21,0.16);
    }
    .lcd-line { white-space: pre; }
    /* ── Wiring / I2C ── */
    .wiring { font-family: "IBM Plex Mono",monospace; white-space: pre-wrap; line-height: 1.45; font-size: 13px; }
    .toggle-grid { display:grid; grid-template-columns:repeat(auto-fit,minmax(140px,1fr)); gap:8px; margin:10px 0 8px; }
    .device-toggle { display:flex; align-items:center; gap:8px; padding:8px 10px; border:1px solid var(--line); border-radius:10px; background:var(--paper); font-size:12px; color:var(--ink); }
    .device-toggle input { accent-color:#4caf50; }
    .ops { margin: 0; padding: 0; list-style: none; font-family: "IBM Plex Mono",monospace; font-size: 13px; }
    .ops li { padding: 8px 0; border-bottom: 1px solid var(--line); }
    /* ── IMU axes ── */
    .axis { display: grid; grid-template-columns: repeat(3,1fr); gap: 8px; }
    .axis div { padding: 10px; border-radius: 14px; background: rgba(217,95,61,0.07); }
    /* ── Motor ── */
    .motor { display: grid; grid-template-columns: repeat(2,1fr); gap: 12px; }
    /* ── Hardware Simulation ── */
    .hw-sim-grid {
      display: grid;
      grid-template-columns: repeat(4,1fr);
      gap: 24px;
      align-items: start;
      justify-items: center;
    }
    .hw-item { display: flex; flex-direction: column; align-items: center; gap: 8px; }
    .hw-name {
      font-size: 11px; color: var(--muted);
      text-transform: uppercase; letter-spacing: .08em;
    }
    /* ── Footer ── */
    .footer { margin-top: 18px; color: var(--muted); font-size: 13px; }
    /* ── E2E Test Runner ── */
    .test-run-btn {
      font-family: inherit; font-size: 13px;
      border: 1px solid var(--line); border-radius: 8px;
      padding: 6px 14px; background: rgba(15,124,107,0.1); color: var(--ink);
      cursor: pointer; transition: background 0.2s; margin-bottom: 10px;
    }
    .test-run-btn:hover  { background: rgba(15,124,107,0.22); }
    .test-run-btn:disabled { opacity: 0.45; cursor: default; }
    #test-output {
      background: #0d1117; color: #c9d1d9;
      font-family: "IBM Plex Mono","Menlo",monospace; font-size: 0.73rem;
      height: 220px; overflow-y: auto; padding: 8px 10px;
      border-radius: 10px; border: 1px solid var(--line);
    }
    #test-output .tpass { color: #4caf50; }
    #test-output .tfail { color: #f44336; }
    #test-output .twarn { color: #ffa726; }
    #test-output .tdone { color: #7e57c2; font-weight: bold; }
    /* ── Responsive ── */
    @media (max-width: 980px) {
      .hero,.grid { grid-template-columns: 1fr; }
      .span-4,.span-6,.span-8,.span-12 { grid-column: span 1; }
      .metric-row,.motor,.axis { grid-template-columns: 1fr; }
      .hw-sim-grid { grid-template-columns: repeat(2,1fr); }
    }
    /* ── Wiring Editor ── */
    .we-chip { display:block; padding:5px 8px; border:1px solid; border-radius:6px; font-size:11px; font-weight:600; cursor:grab; text-align:center; user-select:none; transition:background .15s,opacity .15s; }
    .we-chip:hover { opacity:.8; background:rgba(15,124,107,.08); }
    .we-node { position:absolute; background:var(--paper); border:2px solid; border-radius:8px; min-width:110px; user-select:none; z-index:10; box-shadow:var(--shadow); }
    .we-node-hdr { padding:3px 8px; border-radius:5px 5px 0 0; font-size:11px; font-weight:700; cursor:move; color:#fff; }
    .we-port { display:flex; align-items:center; gap:5px; padding:2px 8px; font-size:10px; cursor:pointer; transition:background .12s; }
    .we-port:hover { background:rgba(127,127,127,.12); }
    .we-port.pend .we-dot { box-shadow:0 0 7px 2px #ff9800 !important; }
    .we-dot { width:9px; height:9px; border-radius:50%; border:1.5px solid rgba(255,255,255,.45); flex-shrink:0; }
    #we-canvas { background-image:radial-gradient(circle,rgba(127,127,127,.2) 1px,transparent 1px); background-size:20px 20px; }
  </style>
</head>
<body>
  <main class="shell">

    <!-- Status / control bar -->
    <div class="status-bar">
      <span class="sdot" id="sdot"></span>
      <span class="stext" id="stext">connecting&#x2026;</span>
      <span class="serr"  id="serr"></span>
      <div class="ctrls">
        <label for="isel" style="color:var(--muted)">Refresh</label>
        <select id="isel">
          <option value="250">250 ms</option>
          <option value="500" selected>500 ms</option>
          <option value="1000">1 s</option>
          <option value="2000">2 s</option>
          <option value="5000">5 s</option>
        </select>
        <button class="btn" id="pbtn">&#x23F8; Pause</button>
        <button class="btn" id="tbtn">&#x1F319; Dark</button>
      </div>
    </div>

    <!-- Hero -->
    <section class="hero">
      <article class="panel hero-main">
        <div class="kicker">mcu-hal-sim-rs</div>
        <h1 class="hero-title" id="board-name">Device Dashboard</h1>
        <p class="hero-sub">
          Reference-path GUI for climate, distance, IMU, servo, and motor-driver simulation.
          The page receives real-time push updates from the host simulator via Server-Sent Events.
        </p>
        <div class="hero-meta">
          <span class="chip" id="mcu-name">MCU</span>
          <span class="chip" id="tick-chip">tick=0</span>
          <span class="chip" id="i2c-chip">i2c ops=0</span>
        </div>
      </article>
      <aside class="panel hero-side">
        <div>
          <div class="label">Distance</div>
          <div class="big" id="distance-value">-- mm</div>
          <div class="label">Servo</div>
          <div class="big" id="servo-value">-- deg</div>
        </div>
        <div class="label">SSE live updates via <code>/api/events</code></div>
      </aside>
    </section>

    <!-- Grid -->
    <section class="grid">

      <!-- Climate -->
      <article class="panel card span-6" id="climate-card">
        <h2>Climate</h2>
        <div class="metric-row">
          <div class="metric">
            <div class="name">Temp</div>
            <div class="val" id="temp-value">--</div>
            <div class="spark-wrap">
              <svg class="spark" id="spark-temp" viewBox="0 0 100 30" preserveAspectRatio="none">
                <polyline points=""/>
              </svg>
            </div>
          </div>
          <div class="metric">
            <div class="name">Humidity</div>
            <div class="val" id="hum-value">--</div>
            <div class="spark-wrap">
              <svg class="spark" id="spark-hum" viewBox="0 0 100 30" preserveAspectRatio="none">
                <polyline points=""/>
              </svg>
            </div>
          </div>
          <div class="metric">
            <div class="name">Pressure</div>
            <div class="val" id="press-value">--</div>
            <div class="spark-wrap">
              <svg class="spark" id="spark-press" viewBox="0 0 100 30" preserveAspectRatio="none">
                <polyline points=""/>
              </svg>
            </div>
          </div>
        </div>
      </article>

      <!-- LCD -->
      <article class="panel card span-6" id="lcd-card">
        <h2>LCD</h2>
        <div class="lcd">
          <div class="lcd-line" id="lcd-line-1">                </div>
          <div class="lcd-line" id="lcd-line-2">                </div>
        </div>
        <div class="footer">Physical LCD frame decoded from backpack traffic.</div>
      </article>

      <!-- HC-SR04 -->
      <article class="panel card span-4" id="distance-card">
        <h2>HC-SR04</h2>
        <div class="metric">
          <div class="name" id="distance-sensor-name">Distance Sensor</div>
          <div class="val" id="distance-metric">-- mm</div>
          <div class="spark-wrap">
            <svg class="spark" id="spark-dist" viewBox="0 0 100 30" preserveAspectRatio="none">
              <polyline points=""/>
            </svg>
          </div>
        </div>
        <!-- Sonar beam visualization -->
        <svg id="sonar-svg" viewBox="0 0 200 76" width="100%" height="76"
             style="margin-top:12px;overflow:visible">
          <rect x="2" y="23" width="28" height="30" rx="3" fill="#1e2a3a" stroke="#445"/>
          <circle cx="10" cy="38" r="5.5" fill="#1a3a5a" stroke="#5599cc"/>
          <circle cx="22" cy="38" r="5.5" fill="#1a3a5a" stroke="#5599cc"/>
          <!-- Ping rings (SMIL animation) -->
          <circle cx="35" cy="38" r="8" fill="none" stroke="#4fc3f7" stroke-width="1.5">
            <animate attributeName="r" from="8" to="54" dur="2s" repeatCount="indefinite" begin="0s"/>
            <animate attributeName="opacity" from="0.7" to="0" dur="2s" repeatCount="indefinite" begin="0s"/>
          </circle>
          <circle cx="35" cy="38" r="8" fill="none" stroke="#4fc3f7" stroke-width="1.2">
            <animate attributeName="r" from="8" to="54" dur="2s" repeatCount="indefinite" begin="0.7s"/>
            <animate attributeName="opacity" from="0.6" to="0" dur="2s" repeatCount="indefinite" begin="0.7s"/>
          </circle>
          <circle cx="35" cy="38" r="8" fill="none" stroke="#4fc3f7" stroke-width="0.8">
            <animate attributeName="r" from="8" to="54" dur="2s" repeatCount="indefinite" begin="1.4s"/>
            <animate attributeName="opacity" from="0.5" to="0" dur="2s" repeatCount="indefinite" begin="1.4s"/>
          </circle>
          <line x1="35" y1="38" id="sonar-beam" x2="135" y2="38"
                stroke="#4fc3f7" stroke-width="1" stroke-dasharray="5 3" opacity="0.55"/>
          <circle id="sonar-echo" cx="135" cy="38" r="5.5" fill="#66bb6a"/>
          <text id="sonar-dist-lbl" x="110" y="14" text-anchor="middle"
                fill="#888" font-size="10" font-family="system-ui">-- mm</text>
        </svg>
      </article>

      <!-- MPU6050 -->
      <article class="panel card span-4" id="imu-card">
        <h2>MPU6050</h2>
        <div class="axis">
          <div><div class="name">Accel X</div><div class="val" id="accel-x">--</div></div>
          <div><div class="name">Accel Y</div><div class="val" id="accel-y">--</div></div>
          <div><div class="name">Accel Z</div><div class="val" id="accel-z">--</div></div>
          <div><div class="name">Gyro X</div><div class="val" id="gyro-x">--</div></div>
          <div><div class="name">Gyro Y</div><div class="val" id="gyro-y">--</div></div>
          <div><div class="name">Gyro Z</div><div class="val" id="gyro-z">--</div></div>
        </div>
        <div class="spark-wrap" style="margin-top:10px">
          <div class="name" style="color:var(--muted);font-size:11px;margin-bottom:3px">Accel Z (mg)</div>
          <svg class="spark" id="spark-accelz" viewBox="0 0 100 30" preserveAspectRatio="none">
            <polyline points=""/>
          </svg>
        </div>
        <!-- Bubble level tilt indicator -->
        <svg id="imu-level-svg" viewBox="0 0 100 100" width="88" height="88"
             style="margin:8px auto 0;display:block">
          <circle cx="50" cy="50" r="42" fill="rgba(0,0,0,0.15)" stroke="#445" stroke-width="1.5"/>
          <circle cx="50" cy="50" r="28" fill="none" stroke="#334" stroke-width="1" stroke-dasharray="3 3"/>
          <circle cx="50" cy="50" r="3.5" fill="none" stroke="#556" stroke-width="1.5"/>
          <line x1="50" y1="13" x2="50" y2="87" stroke="#2a3a3a" stroke-width="1" stroke-dasharray="2 4"/>
          <line x1="13" y1="50" x2="87" y2="50" stroke="#2a3a3a" stroke-width="1" stroke-dasharray="2 4"/>
          <circle id="imu-bubble" cx="50" cy="50" r="9"
                  fill="#4fc3f7" fill-opacity="0.65" stroke="#81d4fa" stroke-width="1.5"/>
          <text x="90" y="53" fill="#445" font-size="8" font-family="system-ui">X</text>
          <text x="44" y="9"  fill="#445" font-size="8" font-family="system-ui">Y</text>
        </svg>
      </article>

      <!-- Motor Driver -->
      <article class="panel card span-4" id="motor-card">
        <h2>Motor Driver</h2>
        <div class="motor">
          <div class="metric">
            <div class="name">Left</div>
            <div class="val" id="motor-left">--</div>
          </div>
          <div class="metric">
            <div class="name">Right</div>
            <div class="val" id="motor-right">--</div>
          </div>
        </div>
      </article>

      <!-- Light Sensor (BH1750) -->
      <article class="panel card span-4" id="light-card">
        <h2 id="light-sensor-name">BH1750</h2>
        <div class="metric">
          <div class="name">Illuminance</div>
          <div class="val" id="light-lux">-- lx</div>
        </div>
        <div class="spark-wrap" style="margin-top:10px">
          <div class="name" style="color:var(--muted);font-size:11px;margin-bottom:3px">Lux history</div>
          <svg class="spark" id="spark-lux" viewBox="0 0 100 30" preserveAspectRatio="none">
            <polyline points=""/>
          </svg>
        </div>
        <div style="font-size:11px;color:var(--muted);margin-top:6px" id="light-raw">raw lux×100: --</div>
      </article>

      <!-- Camera (ESP32-CAM) -->
      <article class="panel card span-4" id="camera-card">
        <h2 id="camera-sensor-name">ESP32-CAM</h2>
        <div class="metric">
          <div class="name">Resolution</div>
          <div class="val" id="camera-resolution">--×--</div>
        </div>
        <div class="metric">
          <div class="name">Frame #</div>
          <div class="val" id="camera-sequence">--</div>
        </div>
        <div style="font-size:11px;color:var(--muted);margin-top:6px">Metadata only (no pixel buffer)</div>
      </article>

      <!-- Gas Sensor (SGP30) -->
      <article class="panel card span-4" id="gas-card">
        <h2 id="gas-sensor-name">SGP30</h2>
        <div class="metric">
          <div class="name">CO&#x2082;</div>
          <div class="val" id="gas-co2">-- ppm</div>
        </div>
        <div class="metric">
          <div class="name">TVOC</div>
          <div class="val" id="gas-voc">-- ppb</div>
        </div>
        <div style="font-size:11px;color:var(--muted);margin-top:6px">Air quality sensor (I2C 0x58)</div>
      </article>

      <!-- RTC (DS3231) -->
      <article class="panel card span-4" id="rtc-card">
        <h2 id="rtc-sensor-name">DS3231</h2>
        <div class="metric">
          <div class="name">DateTime</div>
          <div class="val" id="rtc-datetime" style="font-size:1rem">--</div>
        </div>
        <div style="font-size:11px;color:var(--muted);margin-top:6px">High-precision RTC (I2C 0x68)</div>
      </article>

      <!-- ToF Distance (VL53L0X) -->
      <article class="panel card span-4" id="tof-card">
        <h2 id="tof-sensor-name">VL53L0X</h2>
        <div class="metric">
          <div class="name">Distance</div>
          <div class="val" id="tof-distance">-- mm</div>
        </div>
        <div style="font-size:11px;color:var(--muted);margin-top:6px">Time-of-Flight sensor (I2C 0x29)</div>
      </article>

      <!-- Hardware Simulation -->
      <article class="panel card span-12">
        <h2>Hardware Simulation</h2>
        <div class="hw-sim-grid">

          <!-- LED -->
          <div class="hw-item" id="led-hw-item">
            <div class="hw-name">&#x1F4A1; LED</div>
            <svg id="led-svg" viewBox="0 0 80 100" width="80" height="100">
              <defs>
                <radialGradient id="led-gon" cx="38%" cy="35%" r="60%">
                  <stop offset="0%" stop-color="#fff8a0"/>
                  <stop offset="100%" stop-color="#ffcc00"/>
                </radialGradient>
                <radialGradient id="led-goff" cx="38%" cy="35%" r="60%">
                  <stop offset="0%" stop-color="#4a3a10"/>
                  <stop offset="100%" stop-color="#1a1200"/>
                </radialGradient>
                <filter id="led-glow" x="-100%" y="-100%" width="300%" height="300%">
                  <feGaussianBlur in="SourceGraphic" stdDeviation="6" result="blur"/>
                  <feMerge>
                    <feMergeNode in="blur"/>
                    <feMergeNode in="blur"/>
                    <feMergeNode in="SourceGraphic"/>
                  </feMerge>
                </filter>
              </defs>
              <circle id="led-body" cx="40" cy="40" r="24"
                      fill="url(#led-goff)" stroke="#666" stroke-width="1.5"/>
              <ellipse id="led-hl" cx="34" cy="32" rx="7" ry="5"
                       fill="white" fill-opacity="0"/>
              <line x1="32" y1="64" x2="32" y2="80"
                    stroke="#888" stroke-width="2.5" stroke-linecap="round"/>
              <line x1="48" y1="64" x2="48" y2="80"
                    stroke="#888" stroke-width="2.5" stroke-linecap="round"/>
              <text x="27" y="92" fill="#666" font-size="11" font-family="monospace">+</text>
              <text x="44" y="92" fill="#666" font-size="11" font-family="monospace">&#x2212;</text>
              <text id="led-lbl" x="40" y="100" text-anchor="middle"
                    fill="#666" font-size="9" font-family="system-ui">OFF</text>
            </svg>
          </div>

          <!-- Servo -->
          <div class="hw-item" id="servo-hw-item">
            <div class="hw-name">&#x2699; Servo</div>
            <svg id="servo-svg" viewBox="0 0 120 114" width="120" height="114">
              <rect x="20" y="62" width="80" height="42" rx="6" fill="#1e2a3a" stroke="#445"/>
              <rect x="9"  y="68" width="14" height="28" rx="3" fill="#1e2a3a" stroke="#445"/>
              <rect x="97" y="68" width="14" height="28" rx="3" fill="#1e2a3a" stroke="#445"/>
              <circle cx="16"  cy="82" r="3" fill="none" stroke="#445"/>
              <circle cx="104" cy="82" r="3" fill="none" stroke="#445"/>
              <circle cx="60"  cy="62" r="10" fill="#2e3d4d" stroke="#556"/>
              <circle cx="60"  cy="62" r="4"  fill="#607890"/>
              <!-- Arm rotates around shaft pivot (60, 62) -->
              <g id="servo-arm" transform="rotate(0 60 62)">
                <rect x="57.5" y="16" width="5" height="46" rx="2.5" fill="#4fc3f7"/>
                <circle cx="60" cy="16" r="5.5" fill="#4fc3f7"/>
                <circle cx="60" cy="62" r="3.5" fill="#81d4fa"/>
              </g>
              <text id="servo-lbl" x="60" y="112" text-anchor="middle"
                    fill="#888" font-size="11" font-family="system-ui">90&#xB0;</text>
            </svg>
          </div>

          <!-- Motor Left -->
          <div class="hw-item" id="motor-left-item">
            <div class="hw-name">&#x1F504; Motor L</div>
            <svg viewBox="0 0 90 104" width="90" height="104">
              <circle cx="45" cy="45" r="36" fill="#111820" stroke="#334" stroke-width="1.5"/>
              <g id="m-left-rotor">
                <line x1="45" y1="16" x2="45" y2="74" stroke="#2a4a2a" stroke-width="2"/>
                <line x1="16" y1="45" x2="74" y2="45" stroke="#2a4a2a" stroke-width="2"/>
                <circle cx="45" cy="16" r="4.5" fill="#66bb6a"/>
                <circle cx="74" cy="45" r="4.5" fill="#66bb6a"/>
                <circle cx="45" cy="74" r="4.5" fill="#66bb6a"/>
                <circle cx="16" cy="45" r="4.5" fill="#66bb6a"/>
              </g>
              <circle cx="45" cy="45" r="7"   fill="#334" stroke="#556"/>
              <circle cx="45" cy="45" r="3"   fill="#778"/>
              <rect x="9"  y="88" width="72" height="6" rx="3" fill="#1a2020"/>
              <rect id="m-left-bar" x="9" y="88" width="0" height="6" rx="3" fill="#66bb6a"/>
              <text id="m-left-lbl" x="45" y="103" text-anchor="middle"
                    fill="#888" font-size="9" font-family="system-ui">--</text>
            </svg>
          </div>

          <!-- Motor Right -->
          <div class="hw-item" id="motor-right-item">
            <div class="hw-name">&#x1F504; Motor R</div>
            <svg viewBox="0 0 90 104" width="90" height="104">
              <circle cx="45" cy="45" r="36" fill="#111820" stroke="#334" stroke-width="1.5"/>
              <g id="m-right-rotor">
                <line x1="45" y1="16" x2="45" y2="74" stroke="#2a4a2a" stroke-width="2"/>
                <line x1="16" y1="45" x2="74" y2="45" stroke="#2a4a2a" stroke-width="2"/>
                <circle cx="45" cy="16" r="4.5" fill="#66bb6a"/>
                <circle cx="74" cy="45" r="4.5" fill="#66bb6a"/>
                <circle cx="45" cy="74" r="4.5" fill="#66bb6a"/>
                <circle cx="16" cy="45" r="4.5" fill="#66bb6a"/>
              </g>
              <circle cx="45" cy="45" r="7"   fill="#334" stroke="#556"/>
              <circle cx="45" cy="45" r="3"   fill="#778"/>
              <rect x="9"  y="88" width="72" height="6" rx="3" fill="#1a2020"/>
              <rect id="m-right-bar" x="9" y="88" width="0" height="6" rx="3" fill="#66bb6a"/>
              <text id="m-right-lbl" x="45" y="103" text-anchor="middle"
                    fill="#888" font-size="9" font-family="system-ui">--</text>
            </svg>
          </div>

        </div>
      </article>

      <!-- Wiring -->
      <article class="panel card span-8">
        <h2 style="display:flex;align-items:center;gap:12px">
          Wiring Diagram
          <select id="board-select" style="font-size:12px;background:#1a2a1a;color:#7bc47b;border:1px solid #3d7a3d;border-radius:4px;padding:2px 6px;cursor:pointer">
            <option value="original-esp32">Original ESP32</option>
            <option value="arduino-nano">Arduino Nano</option>
          </select>
          <select id="sensor-profile-select" style="font-size:12px;background:#1a2a1a;color:#7bc47b;border:1px solid #3d7a3d;border-radius:4px;padding:2px 6px;cursor:pointer">
          </select>
        </h2>
        <div id="device-toggle-list" class="toggle-grid"></div>
        <label class="device-toggle" style="margin-top:10px;display:inline-flex">
          <input id="show-bus-labels-toggle" type="checkbox" />
          <span>Show bus labels</span>
        </label>
        <div id="wiring-svg-wrap" style="width:100%;overflow-x:auto;min-height:180px"></div>
        <div class="footer" style="margin-top:6px">
          Attached: <span id="wiring-devices" style="font-family:monospace">--</span>
        </div>
      </article>

      <!-- I2C Activity -->
      <article class="panel card span-4">
        <h2>I2C Activity</h2>
        <ul class="ops" id="i2c-ops"></ul>
      </article>

      <!-- Diagnostics -->
      <article class="panel card span-8" id="diagnostics-panel">
        <h2>&#x1F6A7; Diagnostics</h2>
        <div style="display:flex;align-items:baseline;gap:8px;margin-bottom:8px">
          <span style="font-size:11px;color:var(--muted)">Total events:</span>
          <span id="diag-event-count" style="font-weight:600;font-variant-numeric:tabular-nums">0</span>
        </div>
        <ul id="diag-events" style="margin:0;padding:0;list-style:none;font-size:12px;font-family:'IBM Plex Mono',monospace;max-height:160px;overflow-y:auto"></ul>
      </article>

      <!-- E2E Test Runner -->
      <article class="panel card span-12">
        <h2>&#x1F9EA; E2E Test Runner</h2>
        <button class="test-run-btn" id="run-tests-btn" onclick="runTests()">&#x25B6; Run Tests (cargo test --workspace)</button>
        <div id="test-output"></div>
      </article>

      <!-- Wiring Editor -->
      <article class="panel card span-12" style="padding:0;overflow:hidden">
        <div style="padding:13px 20px;border-bottom:1px solid var(--line);display:flex;align-items:center;gap:10px;flex-wrap:wrap">
          <h2 style="margin:0">&#x1F50C; Wiring Editor</h2>
          <span style="font-size:12px;color:var(--muted)">Drag devices &#x2192; canvas &bull; click ports to wire &bull; click wire to delete</span>
          <div style="margin-left:auto;display:flex;gap:6px">
            <button class="btn" onclick="weExport()">&#x1F4BE; Export</button>
            <button class="btn" onclick="weImport()">&#x1F4C2; Import</button>
            <button class="btn" onclick="weClear()">&#x1F5D1; Clear</button>
          </div>
        </div>
        <div style="display:flex;height:400px">
          <div id="we-sidebar" style="width:132px;flex-shrink:0;border-right:1px solid var(--line);padding:8px;overflow-y:auto;display:flex;flex-direction:column;gap:5px">
            <div style="font-size:10px;color:var(--muted);text-transform:uppercase;letter-spacing:.07em;margin-bottom:2px">Library</div>
          </div>
          <div id="we-canvas" style="flex:1;position:relative;overflow:hidden"
               ondragover="event.preventDefault()"
               ondrop="weOnDrop(event)">
            <svg id="we-svg" style="position:absolute;inset:0;width:100%;height:100%;overflow:visible"></svg>
          </div>
        </div>
        <div id="we-status" style="padding:5px 14px;border-top:1px solid var(--line);font-size:11px;color:var(--muted)">Ready &#x2014; drag a device chip to the canvas to get started.</div>
      </article>

      <!-- ESP32 Flash -->
      <article class="panel card span-12">
        <h2>&#x26A1; ESP32 Flash</h2>
        <div style="display:flex;align-items:center;gap:8px;flex-wrap:wrap;margin-bottom:10px">
          <select id="flash-port" style="min-width:220px">
            <option value="">-- select port --</option>
          </select>
          <button class="btn" onclick="flashRefreshPorts()">&#x1F504; Refresh</button>
          <input id="flash-bin" type="text" placeholder="path/to/firmware.elf (optional)"
                 style="flex:1;min-width:200px;background:var(--paper);color:var(--ink);border:1px solid var(--line);border-radius:8px;padding:5px 10px;font-size:13px;font-family:inherit"/>
          <button class="btn" id="flash-btn" onclick="flashStart()">&#x26A1; Flash</button>
        </div>
        <div id="flash-output" class="wiring"
             style="background:#0a0a0a;color:#aaffaa;border-radius:10px;padding:10px 14px;font-size:12px;min-height:80px;max-height:260px;overflow-y:auto;white-space:pre-wrap;font-family:'IBM Plex Mono',monospace"></div>
        <div id="flash-status" style="margin-top:6px;font-size:11px;color:var(--muted)">Ready.</div>
      </article>

    </section>
  </main>

  <script>
    // ── History ring buffers ──
    const HIST = 60;
    const hist = { temp:[], hum:[], press:[], dist:[], accelz:[], lux:[] };
    function push(key, v) {
      if (v == null) return;
      hist[key].push(v);
      if (hist[key].length > HIST) hist[key].shift();
    }

    // ── SVG sparkline ──
    function sparkline(id, data) {
      if (data.length < 2) return;
      const svg = document.getElementById(id);
      if (!svg) return;
      const W = 100, H = 30, PAD = 0.1;
      const lo = Math.min(...data), hi = Math.max(...data);
      const range = hi - lo || 1;
      const pts = data.map((v, i) => {
        const x = (i / (data.length - 1)) * W;
        const y = H - PAD * H - ((v - lo) / range) * H * (1 - 2 * PAD);
        return x.toFixed(1) + "," + y.toFixed(1);
      }).join(" ");
      svg.querySelector("polyline").setAttribute("points", pts);
    }

    // ── Status bar ──
    let lastOkMs = null, errCount = 0;
    function clearErr() {
      document.getElementById("serr").textContent = "";
    }
    function setOk() {
      const now = Date.now();
      const ago = lastOkMs ? (now - lastOkMs) + " ms ago" : "just now";
      lastOkMs = now; errCount = 0;
      document.getElementById("sdot").className = "sdot ok";
      document.getElementById("stext").textContent = "Online \u00B7 updated " + ago;
    }
    function setErr(msg) {
      errCount++;
      document.getElementById("sdot").className = "sdot err";
      document.getElementById("stext").textContent = "Error \u00D7" + errCount;
      document.getElementById("serr").textContent = msg || "";
    }

    // ── DOM helpers ──
    const $ = id => document.getElementById(id);
    const fmt = (v, sfx) => v == null ? "--" : v + sfx;
    const LCD_BLANK = "                ";
    const lcdLines = ["lcd-line-1","lcd-line-2"].map(id => $(id));
    function setSectionVisible(id, visible) {
      const el = $(id);
      if (el) el.hidden = !visible;
    }
    function renderDeviceToggles(devices) {
      const host = $("device-toggle-list");
      if (!host) return;
      host.innerHTML = "";
      devices.forEach(device => {
        const label = document.createElement("label");
        label.className = "device-toggle";
        const input = document.createElement("input");
        input.type = "checkbox";
        input.checked = !!device.enabled;
        input.dataset.deviceKind = device.kind;
        input.addEventListener("change", changeDeviceToggle);
        const text = document.createElement("span");
        text.textContent = device.label;
        label.appendChild(input);
        label.appendChild(text);
        host.appendChild(label);
      });
    }
    function applyDeviceSelection(selectedDevices) {
      const enabled = new Set(selectedDevices || []);
      setSectionVisible("distance-card", enabled.has("hc_sr04"));
      setSectionVisible("imu-card", enabled.has("mpu6050"));
      setSectionVisible("motor-card", enabled.has("l298n"));
      setSectionVisible("light-card", enabled.has("bh1750"));
      setSectionVisible("camera-card", enabled.has("esp32_cam"));
      setSectionVisible("gas-card", enabled.has("sgp30"));
      setSectionVisible("rtc-card", enabled.has("ds3231"));
      setSectionVisible("tof-card", enabled.has("vl53l0x"));
      setSectionVisible("servo-hw-item", enabled.has("servo"));
      setSectionVisible("motor-left-item", enabled.has("l298n"));
      setSectionVisible("motor-right-item", enabled.has("l298n"));
      return enabled;
    }

    // ── Motor animation (requestAnimationFrame loop) ──
    const mAngle = { left: 0, right: 0 };
    const mState = {
      left:  { run: false, spd: 0, dir: 1 },
      right: { run: false, spd: 0, dir: 1 }
    };
    (function motorLoop() {
      for (const side of ["left", "right"]) {
        if (!mState[side].run) continue;
        mAngle[side] = (mAngle[side] + mState[side].spd * mState[side].dir + 360) % 360;
        const r = $("m-" + side + "-rotor");
        if (r) r.setAttribute("transform",
          "rotate(" + mAngle[side].toFixed(1) + " 45 45)");
      }
      requestAnimationFrame(motorLoop);
    })();

    // ── LED ──
    function setLed(tick) {
      const on = (tick % 100) < 50;
      const body = $("led-body");
      if (!body) return;
      body.setAttribute("fill", on ? "url(#led-gon)" : "url(#led-goff)");
      body.setAttribute("filter", on ? "url(#led-glow)" : "");
      const hl = $("led-hl");
      if (hl) hl.setAttribute("fill-opacity", on ? "0.38" : "0");
      const lbl = $("led-lbl");
      if (lbl) {
        lbl.textContent = on ? "ON" : "OFF";
        lbl.setAttribute("fill", on ? "#ffdd44" : "#666");
      }
    }

    // ── Servo ──
    function setServo(angleDeg) {
      const arm = $("servo-arm");
      if (arm) arm.setAttribute("transform",
        "rotate(" + (angleDeg - 90) + " 60 62)");
      const lbl = $("servo-lbl");
      if (lbl) lbl.textContent = angleDeg + "\u00B0";
    }

    // ── Motor visual ──
    function setMotorViz(side, dir, duty) {
      const stopped = dir === "brake" || dir === "coast" || duty < 3;
      mState[side].run = !stopped;
      mState[side].spd = stopped ? 0 : (2 + duty * 0.1);
      mState[side].dir = dir === "reverse" ? -1 : 1;
      const bar = $("m-" + side + "-bar");
      if (bar) bar.setAttribute("width", String(Math.round(duty * 72 / 100)));
      const lbl = $("m-" + side + "-lbl");
      if (lbl) {
        const icon = stopped
          ? (dir === "brake" ? "\u25A0" : "\u2014")
          : (dir === "reverse" ? "\u21BA" : "\u21BB");
        lbl.textContent = icon + " " + (stopped ? dir : duty + "%");
      }
    }

    // ── Sonar ──
    function setSonar(distMm) {
      const MAX = 600, X0 = 35, X1 = 188;
      const d = distMm == null ? 300 : distMm;
      const x = (X0 + (Math.min(d, MAX) / MAX) * (X1 - X0)).toFixed(0);
      const beam = $("sonar-beam");
      const echo = $("sonar-echo");
      const txt  = $("sonar-dist-lbl");
      if (beam) beam.setAttribute("x2", x);
      if (echo) {
        echo.setAttribute("cx", x);
        echo.setAttribute("fill",
          d < 200 ? "#ef5350" : d < 400 ? "#ffca28" : "#66bb6a");
      }
      if (txt) txt.textContent = distMm == null ? "-- mm" : distMm + " mm";
    }

    // ── IMU bubble level ──
    function setImuLevel(ax, ay) {
      const bubble = $("imu-bubble");
      if (!bubble) return;
      const s = 30 / 1000;
      const bx = Math.max(14, Math.min(86, 50 + ax * s)).toFixed(1);
      const by = Math.max(14, Math.min(86, 50 - ay * s)).toFixed(1);
      bubble.setAttribute("cx", bx);
      bubble.setAttribute("cy", by);
      const tilt = Math.sqrt(ax * ax + ay * ay);
      bubble.setAttribute("fill",
        tilt > 700 ? "#ef5350" : tilt > 300 ? "#ffca28" : "#4fc3f7");
    }

    // ── Wiring diagram (loaded once at startup, then refreshed every 5s) ──
    function wiringErrorMessage(action, err) {
      return `${action} failed: ${err && err.message ? err.message : "unknown error"}`;
    }
    async function fetchJsonOrThrow(url, action) {
      const response = await fetch(url);
      if (!response.ok) throw new Error(`${action} returned HTTP ${response.status}`);
      return response.json();
    }
    async function fetchTextOrThrow(url, action) {
      const response = await fetch(url);
      if (!response.ok) throw new Error(`${action} returned HTTP ${response.status}`);
      return response.text();
    }
    async function loadWiringDiagram() {
      try {
        const svg = await fetchTextOrThrow("/api/wiring/svg", "load wiring diagram");
        const wrap = $("wiring-svg-wrap");
        if (wrap) {
          wrap.innerHTML = svg;
          clearErr();
        }
      } catch(err) {
        setErr(wiringErrorMessage("Wiring diagram", err));
      }
    }
    async function refreshWiringUi() {
      await loadWiringConfig();
      await loadWiringDiagram();
    }
    let wiringUpdatePromise = Promise.resolve();
    function queueWiringUpdate(task) {
      wiringUpdatePromise = wiringUpdatePromise
        .catch(() => {})
        .then(task);
      return wiringUpdatePromise;
    }
    function changeWiringConfig() {
      const profileSel = $("sensor-profile-select");
      const boardSel = $("board-select");
      const showBusLabelsToggle = $("show-bus-labels-toggle");
      const body = {};
      if (boardSel) body.board = boardSel.value;
      if (profileSel) body.sensor_profile = profileSel.value;
      if (showBusLabelsToggle) body.show_bus_labels = showBusLabelsToggle.checked;
      return queueWiringUpdate(async () => {
        try {
          const response = await fetch("/api/wiring", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(body),
          });
          if (!response.ok) {
            throw new Error(`update returned HTTP ${response.status}`);
          }
          await refreshWiringUi();
          clearErr();
        } catch(err) {
          setErr(wiringErrorMessage("Wiring update", err));
        }
      });
    }
    function changeDeviceToggle() {
      const boardSel = $("board-select");
      const profileSel = $("sensor-profile-select");
      const showBusLabelsToggle = $("show-bus-labels-toggle");
      const selectedDevices = Array.from(document.querySelectorAll('#device-toggle-list input[data-device-kind]:checked'))
        .map(input => input.dataset.deviceKind);
      const body = { selected_devices: selectedDevices };
      if (boardSel) body.board = boardSel.value;
      if (profileSel) body.sensor_profile = profileSel.value;
      if (showBusLabelsToggle) body.show_bus_labels = showBusLabelsToggle.checked;
      return queueWiringUpdate(async () => {
        try {
          const response = await fetch("/api/wiring", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(body),
          });
          if (!response.ok) {
            throw new Error(`update returned HTTP ${response.status}`);
          }
          await refreshWiringUi();
          clearErr();
        } catch(err) {
          setErr(wiringErrorMessage("Device toggle update", err));
        }
      });
    }
    function changeBusLabelToggle() {
      const showBusLabelsToggle = $("show-bus-labels-toggle");
      const body = {
        show_bus_labels: !!(showBusLabelsToggle && showBusLabelsToggle.checked),
      };
      return queueWiringUpdate(async () => {
        try {
          const response = await fetch("/api/wiring", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(body),
          });
          if (!response.ok) {
            throw new Error(`update returned HTTP ${response.status}`);
          }
          await refreshWiringUi();
          clearErr();
        } catch(err) {
          setErr(wiringErrorMessage("Bus label toggle update", err));
        }
      });
    }
    async function loadWiringConfig() {
      try {
        const data = await fetchJsonOrThrow("/api/wiring", "load wiring config");
        const boardSel = $("board-select");
        const profileSel = $("sensor-profile-select");
        const showBusLabelsToggle = $("show-bus-labels-toggle");
        if (boardSel) boardSel.value = data.board === "nano" ? "arduino-nano" : "original-esp32";
        if (profileSel) profileSel.value = data.sensor_profile;
        if (showBusLabelsToggle) showBusLabelsToggle.checked = !!data.show_bus_labels;
        renderDeviceToggles(data.available_devices || []);
        applyDeviceSelection(data.selected_devices || []);
      } catch(err) {
        setErr(wiringErrorMessage("Wiring config", err));
      }
    }
    async function initProfileSelect() {
      try {
        const data = await fetchJsonOrThrow("/api/wiring/profiles", "load wiring profiles");
        const sel = $("sensor-profile-select");
        if (!sel) return;
        sel.innerHTML = data.profiles
          .map(p => `<option value="${p.slug}">${p.name}</option>`)
          .join("");
      } catch(err) {
        setErr(wiringErrorMessage("Wiring profiles", err));
      }
    }
    const boardSel = $("board-select");
    if (boardSel) boardSel.addEventListener("change", changeWiringConfig);
    const profileSel = $("sensor-profile-select");
    if (profileSel) profileSel.addEventListener("change", changeWiringConfig);
    const showBusLabelsToggle = $("show-bus-labels-toggle");
    if (showBusLabelsToggle) showBusLabelsToggle.addEventListener("change", changeBusLabelToggle);
    queueWiringUpdate(async () => {
      await initProfileSelect();
      await refreshWiringUi();
    });
    setInterval(() => {
      queueWiringUpdate(async () => {
        await loadWiringDiagram();
      });
    }, 5000);

    // ── Reactive wiring flash: I2C activity → SDA/SCL glow ──
    let lastI2cFirstOp = '';
    function flashWires() {
      const wrap = $("wiring-svg-wrap");
      if (!wrap) return;
      const wires = wrap.querySelectorAll('.w-sda, .w-scl');
      if (!wires.length) return;
      wires.forEach(el => {
        el.style.transition = '';
        el.style.filter = 'brightness(3) drop-shadow(0 0 4px white)';
        el.style.strokeWidth = '4.5';
      });
      setTimeout(() => {
        wires.forEach(el => {
          el.style.transition = 'filter 0.4s ease-out, stroke-width 0.4s ease-out';
          el.style.filter = '';
          el.style.strokeWidth = '';
        });
      }, 180);
    }

    // ── Main render (called from SSE messages) ──
    let paused = false;
    function renderState(s) {
      if (paused) return;
      const enabled = applyDeviceSelection(s.wiring.selected_devices);
      const bme280Enabled = enabled.has("bme280");
      const lcdEnabled = enabled.has("lcd1602");
      const hcSr04Enabled = enabled.has("hc_sr04");
      const imuEnabled = enabled.has("mpu6050");
      const servoEnabled = enabled.has("servo");
      const motorEnabled = enabled.has("l298n");
      const lightEnabled = enabled.has("bh1750");
      const cameraEnabled = enabled.has("esp32_cam");
      const gasEnabled = enabled.has("sgp30");
      const rtcEnabled = enabled.has("ds3231");
      const tofEnabled = enabled.has("vl53l0x");

      $("board-name").textContent = s.board_name;
      $("mcu-name").textContent   = s.mcu_name;
      $("tick-chip").textContent  = "tick=" + s.tick;
      $("i2c-chip").textContent   = "i2c ops=" + s.i2c.operation_count;

      $("temp-value").textContent  = bme280Enabled ? fmt(s.climate.temperature_c,    " \u00B0C") : "--";
      $("hum-value").textContent   = bme280Enabled ? fmt(s.climate.humidity_percent, " %") : "--";
      $("press-value").textContent = bme280Enabled ? fmt(s.climate.pressure_pa,      " Pa") : "--";
      lcdLines[0].textContent = lcdEnabled ? s.climate.physical_lcd_frame[0] : LCD_BLANK;
      lcdLines[1].textContent = lcdEnabled ? s.climate.physical_lcd_frame[1] : LCD_BLANK;

      $("distance-value").textContent       = hcSr04Enabled ? fmt(s.distance.distance_mm, " mm") : "-- mm";
      $("distance-metric").textContent      = hcSr04Enabled ? fmt(s.distance.distance_mm, " mm") : "-- mm";
      $("distance-sensor-name").textContent = s.distance.sensor_name;
      $("servo-value").textContent          = servoEnabled ? s.servo.angle_degrees + " deg" : "-- deg";

      $("accel-x").textContent = imuEnabled ? s.imu.accel_mg[0]  + " mg" : "--";
      $("accel-y").textContent = imuEnabled ? s.imu.accel_mg[1]  + " mg" : "--";
      $("accel-z").textContent = imuEnabled ? s.imu.accel_mg[2]  + " mg" : "--";
      $("gyro-x").textContent  = imuEnabled ? s.imu.gyro_mdps[0] + " mdps" : "--";
      $("gyro-y").textContent  = imuEnabled ? s.imu.gyro_mdps[1] + " mdps" : "--";
      $("gyro-z").textContent  = imuEnabled ? s.imu.gyro_mdps[2] + " mdps" : "--";

      $("motor-left").textContent  = motorEnabled ? (s.motor_driver.left.direction  + " " + s.motor_driver.left.duty_percent  + "%") : "--";
      $("motor-right").textContent = motorEnabled ? (s.motor_driver.right.direction + " " + s.motor_driver.right.duty_percent + "%") : "--";

      // Light sensor (BH1750)
      if (lightEnabled && s.light) {
        const lux = (s.light.lux_x100 / 100).toFixed(2);
        $("light-lux").textContent = lux + " lx";
        $("light-raw").textContent = "raw lux\u00D7100: " + s.light.lux_x100;
        const el = $("light-sensor-name");
        if (el) el.textContent = s.light.sensor_name;
        push("lux", s.light.lux_x100 / 100);
        sparkline("spark-lux", hist.lux);
      } else {
        $("light-lux").textContent = "-- lx";
        $("light-raw").textContent = "raw lux\u00D7100: --";
      }

      // Camera (ESP32-CAM)
      if (cameraEnabled && s.camera) {
        $("camera-resolution").textContent = s.camera.width + "\u00D7" + s.camera.height;
        $("camera-sequence").textContent   = "#" + s.camera.sequence;
        const el = $("camera-sensor-name");
        if (el) el.textContent = s.camera.sensor_name;
      } else {
        $("camera-resolution").textContent = "--\u00D7--";
        $("camera-sequence").textContent = "--";
      }

      // Gas sensor (SGP30)
      if (gasEnabled && s.gas) {
        $("gas-co2").textContent = s.gas.co2_ppm != null ? s.gas.co2_ppm + " ppm" : "-- ppm";
        $("gas-voc").textContent = s.gas.voc_ppb != null ? s.gas.voc_ppb + " ppb" : "-- ppb";
        const el = $("gas-sensor-name");
        if (el) el.textContent = s.gas.sensor_name;
      } else {
        $("gas-co2").textContent = "-- ppm";
        $("gas-voc").textContent = "-- ppb";
      }

      // RTC (DS3231)
      if (rtcEnabled && s.rtc) {
        $("rtc-datetime").textContent = s.rtc.datetime_str || "--";
        const el = $("rtc-sensor-name");
        if (el) el.textContent = s.rtc.sensor_name;
      } else {
        $("rtc-datetime").textContent = "--";
      }

      // ToF distance (VL53L0X)
      if (tofEnabled && s.tof) {
        $("tof-distance").textContent = s.tof.distance_mm != null ? s.tof.distance_mm + " mm" : "-- mm";
        const el = $("tof-sensor-name");
        if (el) el.textContent = s.tof.sensor_name;
      } else {
        $("tof-distance").textContent = "-- mm";
      }

      const devEl = $("wiring-devices");
      if (devEl) devEl.textContent = s.wiring.attached_devices.join(", ") || "--";

      const ops = $("i2c-ops");
      ops.innerHTML = "";
      for (const line of s.i2c.recent_operations) {
        const li = document.createElement("li");
        li.textContent = line;
        ops.appendChild(li);
      }

      // history + sparklines
      if (bme280Enabled) {
        push("temp", s.climate.temperature_c);
        push("hum", s.climate.humidity_percent);
        push("press", s.climate.pressure_pa);
      }
      if (hcSr04Enabled) {
        push("dist", s.distance.distance_mm);
      }
      if (imuEnabled) {
        push("accelz", s.imu.accel_mg[2]);
      }
      sparkline("spark-temp",   hist.temp);
      sparkline("spark-hum",    hist.hum);
      sparkline("spark-press",  hist.press);
      sparkline("spark-dist",   hist.dist);
      sparkline("spark-accelz", hist.accelz);

      // visual simulation
      setLed(s.tick);
      setServo(s.servo.angle_degrees);
      setMotorViz("left",  s.motor_driver.left.direction,  motorEnabled ? s.motor_driver.left.duty_percent : 0);
      setMotorViz("right", s.motor_driver.right.direction, motorEnabled ? s.motor_driver.right.duty_percent : 0);
      setSonar(hcSr04Enabled ? s.distance.distance_mm : null);
      setImuLevel(imuEnabled ? s.imu.accel_mg[0] : 0, imuEnabled ? s.imu.accel_mg[1] : 0);

      // diagnostics panel
      const diagCount = $("diag-event-count");
      const diagList  = $("diag-events");
      if (diagCount && s.diagnostics) {
        diagCount.textContent = s.diagnostics.event_count;
      }
      if (diagList && s.diagnostics) {
        diagList.innerHTML = "";
        for (const ev of (s.diagnostics.recent_events || [])) {
          const li = document.createElement("li");
          li.style.cssText = "display:flex;gap:6px;align-items:baseline;padding:3px 0;border-bottom:1px solid var(--line);font-size:11px;font-family:'IBM Plex Mono',monospace";

          // severity badge
          const sev = (ev.sev || "info");
          const sevColor = sev === "error" ? "#e55" : sev === "warn" ? "#d90" : "#58a";
          const badge = document.createElement("span");
          badge.textContent = sev.toUpperCase();
          badge.style.cssText = "flex-shrink:0;padding:1px 4px;border-radius:3px;font-size:9px;font-weight:700;letter-spacing:.04em;background:" + sevColor + ";color:#fff";

          // timestamp (ms → seconds elapsed)
          const ts = document.createElement("span");
          ts.textContent = "+" + ((ev.ts || 0) / 1000).toFixed(1) + "s";
          ts.style.cssText = "flex-shrink:0;color:var(--muted);font-size:10px;min-width:48px;text-align:right";

          // message
          const msg = document.createElement("span");
          msg.textContent = ev.msg || "";
          msg.style.cssText = "flex:1;color:var(--fg);word-break:break-all";

          li.appendChild(badge);
          li.appendChild(ts);
          li.appendChild(msg);
          diagList.appendChild(li);
        }
      }

      // flash SDA/SCL wires when a new I2C operation is detected
      const curOp = s.i2c.recent_operations[0] || '';
      if (curOp && curOp !== lastI2cFirstOp) {
        flashWires();
        lastI2cFirstOp = curOp;
      }

      setOk();
    }

    // ── E2E Test Runner ──
    function runTests() {
      const out = $("test-output");
      out.innerHTML = "";
      const btn = $("run-tests-btn");
      btn.disabled = true;
      btn.textContent = "Running\u2026";

      const addLine = (text, cls) => {
        const div = document.createElement("div");
        div.textContent = text;
        if (cls) div.className = cls;
        out.appendChild(div);
        out.scrollTop = out.scrollHeight;
      };

      const es = new EventSource("/api/test/stream");
      es.onmessage = (e) => {
        const line = e.data;
        if (line.startsWith("[DONE]")) {
          es.close();
          btn.disabled = false;
          btn.textContent = "\u25B6 Run Tests (cargo test --workspace)";
          const ok = line.includes("exit=0");
          addLine(ok ? "\u2714 All tests passed" : "\u2718 Tests finished with failures", ok ? "tpass" : "tfail");
          addLine(line, "tdone");
          return;
        }
        if (line.startsWith("[ERROR]")) { addLine(line, "tfail"); return; }
        const lower = line.toLowerCase();
        if (/\bFAILED\b/.test(line) || /^error/.test(lower)) {
          addLine(line, "tfail");
        } else if (/\.\.\. ok$/.test(line) || /^test result: ok/.test(lower)) {
          addLine(line, "tpass");
        } else if (/^warning/.test(lower)) {
          addLine(line, "twarn");
        } else {
          addLine(line, "");
        }
      };
      es.onerror = () => {
        es.close();
        btn.disabled = false;
        btn.textContent = "\u25B6 Run Tests (cargo test --workspace)";
        addLine("[ERROR] connection lost", "tfail");
      };
    }

    // ── Render throttle (client-side) ──
    let renderIntervalMs = 500;
    let lastRenderMs = 0;
    $("isel").addEventListener("change", e => { renderIntervalMs = +e.target.value; });

    const evtSrc = new EventSource('/api/events');

    evtSrc.onopen = () => { setOk(); };

    evtSrc.onmessage = (e) => {
      if (paused) return;
      const now = Date.now();
      if (now - lastRenderMs < renderIntervalMs) return;
      lastRenderMs = now;
      try { renderState(JSON.parse(e.data)); }
      catch(err) { setErr(err.message); }
    };

    evtSrc.onerror = () => { setErr('SSE reconnecting\u2026'); };

    // ── Pause/resume ──
    $("pbtn").addEventListener("click", () => {
      paused = !paused;
      $("pbtn").innerHTML = paused ? "&#x25B6; Resume" : "&#x23F8; Pause";
      if (paused) {
        $("sdot").className = "sdot";
        $("stext").textContent = "Paused";
      }
    });

    // ── Dark mode ──
    function applyTheme(dark) {
      document.documentElement.classList.toggle("dark", dark);
      $("tbtn").innerHTML = dark ? "&#x2600; Light" : "&#x1F319; Dark";
      try { localStorage.setItem("dash-theme", dark ? "dark" : "light"); } catch(_) {}
    }
    $("tbtn").addEventListener("click", () =>
      applyTheme(!document.documentElement.classList.contains("dark"))
    );
    try { applyTheme(localStorage.getItem("dash-theme") === "dark"); } catch(_) {}

    // ── Boot ──
    // ── Wiring Editor ────────────────────────────────────────────────────────
    const WE_DEFS = {
      'ESP32':        { color:'#2196F3', ports:['3V3','GND','GPIO21(SDA)','GPIO22(SCL)','GPIO5(TRIG)','GPIO18(ECHO)','GPIO13(Servo)','GPIO25','GPIO26','GPIO27','GPIO32'] },
      'Arduino Nano': { color:'#1565C0', ports:['5V','3.3V','GND','A4(SDA)','A5(SCL)','D2(TRIG)','D3(ECHO)','D9(Servo)','D5','D6','D7','D10'] },
      'BME280':   { color:'#4CAF50', ports:['VCC','GND','SDA','SCL'] },
      'MPU6050':  { color:'#9C27B0', ports:['VCC','GND','SDA','SCL'] },
      'HC-SR04':  { color:'#FF5722', ports:['VCC','GND','TRIG','ECHO'] },
      'LCD1602':  { color:'#00BCD4', ports:['VCC','GND','SDA','SCL'] },
      'Servo':    { color:'#FF9800', ports:['VCC','GND','PWM'] },
      'L298N':    { color:'#795548', ports:['12V','GND','IN1','IN2','ENA','IN3','IN4','ENB'] },
      'BH1750':   { color:'#FDD835', ports:['VCC','GND','SDA','SCL','ADDR'] },
      'DHT22':    { color:'#26C6DA', ports:['VCC','GND','DATA'] },
      'SSD1306':  { color:'#78909C', ports:['VCC','GND','SDA','SCL'] },
      'ESP32-CAM':{ color:'#E91E63', ports:['5V','GND','U0TXD','U0RXD','GPIO0','GPIO4(FLASH)','VCC(3V3)'] },
    };
    const weS = { nodes:{}, edges:[], seq:1, pending:null };

    (function weInit() {
      const sb = document.getElementById('we-sidebar');
      Object.entries(WE_DEFS).forEach(function([type, def]) {
        const chip = document.createElement('div');
        chip.className = 'we-chip';
        chip.style.borderColor = def.color;
        chip.style.color = def.color;
        chip.textContent = type;
        chip.draggable = true;
        chip.addEventListener('dragstart', function(e) {
          e.dataTransfer.setData('we-type', type);
        });
        sb.appendChild(chip);
      });
    })();

    function weOnDrop(e) {
      const type = e.dataTransfer.getData('we-type');
      if (!type) return;
      const rect = document.getElementById('we-canvas').getBoundingClientRect();
      weAddNode(type, Math.max(0, e.clientX - rect.left - 55), Math.max(0, e.clientY - rect.top - 20));
    }

    function weBuildNodeEl(id, type, x, y) {
      const def = WE_DEFS[type];
      const el = document.createElement('div');
      el.className = 'we-node';
      el.id = 'we-node-' + id;
      el.style.cssText = 'left:' + x + 'px;top:' + y + 'px;border-color:' + def.color;
      const hdr = document.createElement('div');
      hdr.className = 'we-node-hdr';
      hdr.style.background = def.color;
      hdr.textContent = type;
      el.appendChild(hdr);
      def.ports.forEach(function(port) {
        const row = document.createElement('div');
        row.className = 'we-port';
        row.dataset.port = port;
        row.onclick = function() { weClickPort(id, port, row); };
        const dot = document.createElement('span');
        dot.className = 'we-dot';
        dot.style.background = def.color;
        row.appendChild(dot);
        row.appendChild(document.createTextNode('\u00a0' + port));
        el.appendChild(row);
      });
      let ox, oy;
      hdr.addEventListener('mousedown', function(e) {
        e.preventDefault();
        const nd = weS.nodes[id];
        ox = e.clientX - nd.x; oy = e.clientY - nd.y;
        function onMove(ev) {
          nd.x = ev.clientX - ox; nd.y = ev.clientY - oy;
          el.style.left = nd.x + 'px'; el.style.top = nd.y + 'px';
          weRenderEdges();
        }
        function onUp() {
          document.removeEventListener('mousemove', onMove);
          document.removeEventListener('mouseup', onUp);
        }
        document.addEventListener('mousemove', onMove);
        document.addEventListener('mouseup', onUp);
      });
      return el;
    }

    function weAddNode(type, x, y) {
      if (!WE_DEFS[type]) return;
      const id = 'n' + weS.seq++;
      weS.nodes[id] = { id, type, x, y };
      document.getElementById('we-canvas').appendChild(weBuildNodeEl(id, type, x, y));
      weRenderEdges();
      document.getElementById('we-status').textContent = 'Added ' + type + '.';
    }

    function weClickPort(nodeId, port, rowEl) {
      if (!weS.pending) {
        weS.pending = { nodeId, port, rowEl };
        rowEl.classList.add('pend');
        document.getElementById('we-status').textContent =
          port + ' selected \u2014 click another port to connect, or click again to cancel.';
      } else if (weS.pending.nodeId === nodeId && weS.pending.port === port) {
        weS.pending.rowEl.classList.remove('pend');
        weS.pending = null;
        document.getElementById('we-status').textContent = 'Cancelled.';
      } else {
        const fn = weS.pending.nodeId, fp = weS.pending.port, fEl = weS.pending.rowEl;
        fEl.classList.remove('pend');
        weS.pending = null;
        const dup = weS.edges.some(function(e) {
          return (e.from===fn&&e.fromPort===fp&&e.to===nodeId&&e.toPort===port) ||
                 (e.from===nodeId&&e.fromPort===port&&e.to===fn&&e.toPort===fp);
        });
        if (!dup) {
          weS.edges.push({ id:'e'+weS.seq++, from:fn, fromPort:fp, to:nodeId, toPort:port });
        }
        weRenderEdges();
        document.getElementById('we-status').textContent =
          'Connected ' + fp + ' \u2192 ' + port + '. Click a wire to delete it.';
      }
    }

    function wePortPos(nodeId, portName) {
      const nodeEl = document.getElementById('we-node-' + nodeId);
      if (!nodeEl) return { x:0, y:0 };
      const row = nodeEl.querySelector('[data-port="' + portName + '"]');
      if (!row) return { x:0, y:0 };
      const dot = row.querySelector('.we-dot');
      const cr = document.getElementById('we-canvas').getBoundingClientRect();
      const r = dot.getBoundingClientRect();
      return { x: r.left + r.width/2 - cr.left, y: r.top + r.height/2 - cr.top };
    }

    function weRenderEdges() {
      const svg = document.getElementById('we-svg');
      while (svg.firstChild) svg.removeChild(svg.firstChild);
      weS.edges.forEach(function(edge) {
        const p1 = wePortPos(edge.from, edge.fromPort);
        const p2 = wePortPos(edge.to, edge.toPort);
        const mx = (p1.x + p2.x) / 2;
        const d = 'M'+p1.x+','+p1.y+' C'+mx+','+p1.y+' '+mx+','+p2.y+' '+p2.x+','+p2.y;
        const path = document.createElementNS('http://www.w3.org/2000/svg', 'path');
        path.setAttribute('d', d);
        path.setAttribute('stroke', '#7bc47b');
        path.setAttribute('stroke-width', '2');
        path.setAttribute('fill', 'none');
        svg.appendChild(path);
        const hit = document.createElementNS('http://www.w3.org/2000/svg', 'path');
        hit.setAttribute('d', d);
        hit.setAttribute('stroke', 'transparent');
        hit.setAttribute('stroke-width', '10');
        hit.setAttribute('fill', 'none');
        hit.setAttribute('pointer-events', 'stroke');
        hit.style.cursor = 'pointer';
        const eid = edge.id;
        hit.addEventListener('click', function() {
          weS.edges = weS.edges.filter(function(e) { return e.id !== eid; });
          weRenderEdges();
          document.getElementById('we-status').textContent = 'Wire deleted.';
        });
        svg.appendChild(hit);
      });
    }

    async function weExport() {
      const data = JSON.stringify({ nodes: Object.values(weS.nodes), edges: weS.edges });
      try {
        await fetch('/api/wiring/editor', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: data,
        });
        document.getElementById('we-status').textContent = 'Saved to server.';
      } catch(err) {
        document.getElementById('we-status').textContent = 'Export failed: ' + err.message;
      }
    }

    async function weImport() {
      try {
        const r = await fetch('/api/wiring/editor');
        if (!r.ok) { document.getElementById('we-status').textContent = 'No saved data.'; return; }
        weLoadData(await r.json());
        document.getElementById('we-status').textContent = 'Imported from server.';
      } catch(err) {
        document.getElementById('we-status').textContent = 'Import failed: ' + err.message;
      }
    }

    function weLoadData(data) {
      weClear();
      (data.nodes || []).forEach(function(n) {
        if (!WE_DEFS[n.type]) return;
        weS.nodes[n.id] = n;
        const num = parseInt(n.id.replace('n', ''), 10);
        if (!isNaN(num) && num >= weS.seq) weS.seq = num + 1;
        document.getElementById('we-canvas').appendChild(weBuildNodeEl(n.id, n.type, n.x, n.y));
      });
      weS.edges = (data.edges || []).map(function(e) {
        const num = parseInt(e.id.replace('e', ''), 10);
        if (!isNaN(num) && num >= weS.seq) weS.seq = num + 1;
        return e;
      });
      weRenderEdges();
    }

    function weClear() {
      Object.keys(weS.nodes).forEach(function(id) {
        const el = document.getElementById('we-node-' + id);
        if (el) el.parentNode.removeChild(el);
      });
      weS.nodes = {}; weS.edges = []; weS.seq = 1; weS.pending = null;
      const svg = document.getElementById('we-svg');
      while (svg.firstChild) svg.removeChild(svg.firstChild);
      document.getElementById('we-status').textContent = 'Canvas cleared.';
    }
    // ── Boot ──
    // ── ESP32 Flash ──────────────────────────────────────────────────────────
    async function flashRefreshPorts() {
      try {
        const r = await fetch('/api/flash/devices');
        const ports = await r.json();
        const sel = document.getElementById('flash-port');
        sel.innerHTML = '<option value="">-- select port --</option>';
        ports.forEach(function(p) {
          const opt = document.createElement('option');
          opt.value = p; opt.textContent = p;
          sel.appendChild(opt);
        });
        document.getElementById('flash-status').textContent =
          ports.length ? ports.length + ' port(s) found.' : 'No MCU ports detected.';
      } catch(err) {
        document.getElementById('flash-status').textContent = 'Error: ' + err.message;
      }
    }

    function flashStart() {
      const port = document.getElementById('flash-port').value;
      const bin  = document.getElementById('flash-bin').value.trim();
      const out  = document.getElementById('flash-output');
      const btn  = document.getElementById('flash-btn');
      out.textContent = '';
      btn.disabled = true;
      let url = '/api/flash/stream';
      if (port) url += '?port=' + encodeURIComponent(port);
      if (bin)  url += (port ? '&' : '?') + 'bin=' + encodeURIComponent(bin);
      document.getElementById('flash-status').textContent = 'Flashing\u2026';
      const es = new EventSource(url);
      es.onmessage = function(e) {
        if (e.data.startsWith('[DONE]')) {
          es.close(); btn.disabled = false;
          const code = e.data.includes('exit=0') ? 0 : 1;
          document.getElementById('flash-status').textContent =
            code === 0 ? '\u2705 Flash successful.' : '\u274C Flash failed (see output).';
        } else {
          out.textContent += e.data + '\n';
          out.scrollTop = out.scrollHeight;
        }
      };
      es.onerror = function() {
        es.close(); btn.disabled = false;
        document.getElementById('flash-status').textContent = 'Connection error.';
      };
    }

    flashRefreshPorts();
  </script>
</body>
</html>
"##
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_contains_api_endpoint() {
        let html = dashboard_html();
        // The dashboard uses SSE (/api/events); /api/state remains as HTTP fallback.
        assert!(html.contains("/api/events"));
        assert!(!html.contains("/api/ws"));
        assert!(html.contains("Device Dashboard"));
    }

    #[test]
    fn html_contains_dashboard_features() {
        let html = dashboard_html();
        // sparkline SVGs
        assert!(html.contains("spark-temp"));
        assert!(html.contains("spark-hum"));
        assert!(html.contains("spark-press"));
        assert!(html.contains("spark-dist"));
        assert!(html.contains("spark-accelz"));
        // dark mode
        assert!(html.contains("dash-theme"));
        assert!(html.contains("tbtn"));
        // pause/interval controls
        assert!(html.contains("pbtn"));
        assert!(html.contains("isel"));
        // status bar
        assert!(html.contains("sdot"));
        assert!(html.contains("stext"));
        // wiring SVG diagram
        assert!(html.contains("/api/wiring/svg"));
        assert!(html.contains("wiring-svg-wrap"));
        assert!(html.contains("loadWiringDiagram"));
        // sensor profile selector
        assert!(html.contains("sensor-profile-select"));
        assert!(html.contains("initProfileSelect"));
        assert!(html.contains("changeWiringConfig"));
        assert!(html.contains("applyDeviceSelection(data.selected_devices || [])"));
        // E2E test runner
        assert!(html.contains("/api/test/stream"));
        assert!(html.contains("run-tests-btn"));
        assert!(html.contains("runTests"));
        assert!(html.contains("test-output"));
        // Wiring editor
        assert!(html.contains("/api/wiring/editor"));
        assert!(html.contains("we-canvas"));
        assert!(html.contains("we-sidebar"));
        assert!(html.contains("weExport"));
        assert!(html.contains("weImport"));
        assert!(html.contains("weClear"));
        // ESP32 flash panel
        assert!(html.contains("/api/flash/devices"));
        assert!(html.contains("/api/flash/stream"));
        assert!(html.contains("flash-port"));
        assert!(html.contains("flashStart"));
        assert!(html.contains("flashRefreshPorts"));
    }

    #[test]
    fn html_contains_visual_simulation() {
        let html = dashboard_html();
        // LED
        assert!(html.contains("led-body"));
        assert!(html.contains("led-glow"));
        assert!(html.contains("setLed"));
        // Servo
        assert!(html.contains("servo-arm"));
        assert!(html.contains("setServo"));
        // Motors
        assert!(html.contains("m-left-rotor"));
        assert!(html.contains("m-right-rotor"));
        assert!(html.contains("setMotorViz"));
        // Sonar
        assert!(html.contains("sonar-beam"));
        assert!(html.contains("sonar-echo"));
        assert!(html.contains("setSonar"));
        // IMU bubble level
        assert!(html.contains("imu-bubble"));
        assert!(html.contains("setImuLevel"));
    }

    #[test]
    fn html_contains_device_toggle_controls() {
        let html = dashboard_html();
        assert!(html.contains("device-toggle-list"));
        assert!(html.contains("renderDeviceToggles"));
        assert!(html.contains("changeDeviceToggle"));
        assert!(html.contains("show-bus-labels-toggle"));
        assert!(html.contains("changeBusLabelToggle"));
        assert!(html.contains("applyDeviceSelection"));
        assert!(html.contains(r#"id="led-hw-item""#));
        assert!(html.contains(r#"id="servo-hw-item""#));
        assert!(html.contains(r#"id="motor-left-item""#));
        assert!(html.contains(r#"id="motor-right-item""#));
        let servo_pos = html.find(r#"id="servo-hw-item""#).unwrap();
        assert!(html[servo_pos..].contains("servo-svg"));
    }

    #[test]
    fn state_serializes_to_json() {
        let json = state_to_json(&DeviceDashboardState {
            board_name: "Arduino Nano".to_string(),
            mcu_name: "ATmega328P".to_string(),
            tick: 3,
            climate: ClimatePanelState {
                temperature_c: Some(24.8),
                humidity_percent: Some(43.2),
                pressure_pa: Some(101_325),
                app_frame: [
                    "Temp    24.8C   ".to_string(),
                    "Hum     43.2%   ".to_string(),
                ],
                physical_lcd_frame: [
                    "Temp    24.8C   ".to_string(),
                    "Hum     43.2%   ".to_string(),
                ],
            },
            distance: DistancePanelState {
                distance_mm: Some(180),
                sensor_name: "HC-SR04".to_string(),
            },
            imu: ImuPanelState {
                sensor_name: "MPU6050".to_string(),
                accel_mg: [0, 0, 1000],
                gyro_mdps: [0, 0, 0],
                temperature_c: Some(24.5),
            },
            servo: ServoPanelState { angle_degrees: 45 },
            motor_driver: MotorDriverPanelState {
                driver_name: "L298N".to_string(),
                left: MotorChannelState {
                    direction: "forward".to_string(),
                    duty_percent: 40,
                },
                right: MotorChannelState {
                    direction: "forward".to_string(),
                    duty_percent: 40,
                },
            },
            wiring: WiringPanelState {
                sda_pin: "A4".to_string(),
                scl_pin: "A5".to_string(),
                power_pin: "5V".to_string(),
                ground_pin: "GND".to_string(),
                attached_devices: vec!["0x27".to_string(), "0x77".to_string()],
                selected_devices: vec!["bme280".to_string(), "servo".to_string()],
                show_bus_labels: false,
                diagram_lines: vec![],
            },
            i2c: I2cPanelState {
                operation_count: 12,
                recent_operations: vec!["WRITE addr=0x27".to_string()],
            },
            light: LightPanelState {
                lux_x100: 5000,
                sensor_name: "BH1750".to_string(),
            },
            camera: CameraPanelState {
                width: 320,
                height: 240,
                sequence: 1,
                sensor_name: "ESP32-CAM".to_string(),
            },
            gas: GasPanelState {
                co2_ppm: Some(450),
                voc_ppb: Some(25),
                sensor_name: "SGP30".to_string(),
            },
            rtc: RtcPanelState {
                datetime_str: "2025-05-04 12:00:00".to_string(),
                sensor_name: "DS3231".to_string(),
            },
            tof: TofPanelState {
                distance_mm: Some(500),
                sensor_name: "VL53L0X".to_string(),
            },
            diagnostics: DiagnosticsPanelState {
                event_count: 3,
                recent_events: vec![
                    DiagEvent {
                        elapsed_ms: 5000,
                        severity: "info".to_string(),
                        message: "bme280 enabled".to_string(),
                    },
                    DiagEvent {
                        elapsed_ms: 10000,
                        severity: "error".to_string(),
                        message: "[bh1750] read_lux error".to_string(),
                    },
                ],
            },
        });

        assert!(json.contains("\"board_name\":\"Arduino Nano\""));
        assert!(json.contains("\"sensor_name\":\"HC-SR04\""));
        assert!(json.contains("\"operation_count\":12"));
        assert!(json.contains("\"selected_devices\":[\"bme280\",\"servo\"]"));
        // Light and camera panel assertions
        assert!(
            json.contains("\"lux_x100\":5000"),
            "light.lux_x100 missing in JSON"
        );
        assert!(
            json.contains("\"sensor_name\":\"BH1750\""),
            "light.sensor_name missing in JSON"
        );
        assert!(
            json.contains("\"width\":320"),
            "camera.width missing in JSON"
        );
        assert!(
            json.contains("\"height\":240"),
            "camera.height missing in JSON"
        );
        assert!(
            json.contains("\"sequence\":1"),
            "camera.sequence missing in JSON"
        );
        assert!(
            json.contains("\"sensor_name\":\"ESP32-CAM\""),
            "camera.sensor_name missing in JSON"
        );
        // Gas, RTC, ToF panel assertions
        assert!(
            json.contains("\"co2_ppm\":450"),
            "gas.co2_ppm missing in JSON"
        );
        assert!(
            json.contains("\"voc_ppb\":25"),
            "gas.voc_ppb missing in JSON"
        );
        assert!(
            json.contains("\"sensor_name\":\"SGP30\""),
            "gas.sensor_name missing in JSON"
        );
        assert!(
            json.contains("\"datetime_str\":\"2025-05-04 12:00:00\""),
            "rtc.datetime_str missing in JSON"
        );
        assert!(
            json.contains("\"sensor_name\":\"DS3231\""),
            "rtc.sensor_name missing in JSON"
        );
        assert!(
            json.contains("\"distance_mm\":500"),
            "tof.distance_mm missing in JSON"
        );
        assert!(
            json.contains("\"sensor_name\":\"VL53L0X\""),
            "tof.sensor_name missing in JSON"
        );
        // Diagnostics panel assertions
        assert!(
            json.contains("\"event_count\":3"),
            "diagnostics.event_count missing in JSON"
        );
        assert!(
            json.contains("\"sev\":\"info\""),
            "diagnostics.recent_events sev field missing in JSON"
        );
        assert!(
            json.contains("\"msg\":\"bme280 enabled\""),
            "diagnostics.recent_events[0] msg missing in JSON"
        );
        assert!(
            json.contains("\"ts\":5000"),
            "diagnostics.recent_events[0] ts missing in JSON"
        );
        assert!(
            json.contains("\"diagnostics\""),
            "diagnostics key missing in JSON"
        );
    }

    #[test]
    fn html_contains_diagnostics_panel() {
        let html = dashboard_html();
        assert!(
            html.contains("diag-event-count"),
            "diag-event-count element missing"
        );
        assert!(html.contains("diag-events"), "diag-events list missing");
        assert!(html.contains("Diagnostics"), "Diagnostics heading missing");
        assert!(
            html.contains("s.diagnostics"),
            "diagnostics renderState handler missing"
        );
    }
}
