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
    pub diagram_lines: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct I2cPanelState {
    pub operation_count: usize,
    pub recent_operations: Vec<String>,
}

pub fn state_to_json(state: &DeviceDashboardState) -> String {
    let mut output = String::new();
    let _ = write!(
        output,
        "{{\"board_name\":{},\"mcu_name\":{},\"tick\":{},\"climate\":{{\"temperature_c\":{},\"humidity_percent\":{},\"pressure_pa\":{},\"app_frame\":[{},{}],\"physical_lcd_frame\":[{},{}]}},\"distance\":{{\"distance_mm\":{},\"sensor_name\":{}}},\"imu\":{{\"sensor_name\":{},\"accel_mg\":[{},{},{}],\"gyro_mdps\":[{},{},{}],\"temperature_c\":{}}},\"servo\":{{\"angle_degrees\":{}}},\"motor_driver\":{{\"driver_name\":{},\"left\":{{\"direction\":{},\"duty_percent\":{}}},\"right\":{{\"direction\":{},\"duty_percent\":{}}}}},\"wiring\":{{\"sda_pin\":{},\"scl_pin\":{},\"power_pin\":{},\"ground_pin\":{},\"attached_devices\":[{}],\"diagram_lines\":[{}]}},\"i2c\":{{\"operation_count\":{},\"recent_operations\":[{}]}}}}",
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
        join_json_strings(&state.wiring.diagram_lines),
        state.i2c.operation_count,
        join_json_strings(&state.i2c.recent_operations),
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
          The page receives real-time push updates from the host simulator via WebSocket.
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
        <div class="label">WebSocket live updates via <code>/api/ws</code></div>
      </aside>
    </section>

    <!-- Grid -->
    <section class="grid">

      <!-- Climate -->
      <article class="panel card span-6">
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
      <article class="panel card span-6">
        <h2>LCD</h2>
        <div class="lcd">
          <div class="lcd-line" id="lcd-line-1">                </div>
          <div class="lcd-line" id="lcd-line-2">                </div>
        </div>
        <div class="footer">Physical LCD frame decoded from backpack traffic.</div>
      </article>

      <!-- HC-SR04 -->
      <article class="panel card span-4">
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
      <article class="panel card span-4">
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
      <article class="panel card span-4">
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

      <!-- Hardware Simulation -->
      <article class="panel card span-12">
        <h2>Hardware Simulation</h2>
        <div class="hw-sim-grid">

          <!-- LED -->
          <div class="hw-item">
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
          <div class="hw-item">
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
          <div class="hw-item">
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
          <div class="hw-item">
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
        </h2>
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

      <!-- E2E Test Runner -->
      <article class="panel card span-12">
        <h2>&#x1F9EA; E2E Test Runner</h2>
        <button class="test-run-btn" id="run-tests-btn" onclick="runTests()">&#x25B6; Run Tests (cargo test --workspace)</button>
        <div id="test-output"></div>
      </article>

    </section>
  </main>

  <script>
    // ── History ring buffers ──
    const HIST = 60;
    const hist = { temp:[], hum:[], press:[], dist:[], accelz:[] };
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
    function setOk() {
      const now = Date.now();
      const ago = lastOkMs ? (now - lastOkMs) + " ms ago" : "just now";
      lastOkMs = now; errCount = 0;
      document.getElementById("sdot").className = "sdot ok";
      document.getElementById("stext").textContent = "Online \u00B7 updated " + ago;
      document.getElementById("serr").textContent = "";
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
    const lcdLines = ["lcd-line-1","lcd-line-2"].map(id => $(id));

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
    async function loadWiringDiagram() {
      try {
        const r = await fetch("/api/wiring/svg");
        if (!r.ok) return;
        const svg = await r.text();
        const wrap = $("wiring-svg-wrap");
        if (wrap) wrap.innerHTML = svg;
      } catch(_) {}
    }
    async function changeBoard(boardName) {
      try {
        await fetch("/api/wiring", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ board: boardName }),
        });
        await loadWiringDiagram();
      } catch(_) {}
    }
    const boardSel = $("board-select");
    if (boardSel) boardSel.addEventListener("change", () => changeBoard(boardSel.value));
    loadWiringDiagram();
    setInterval(loadWiringDiagram, 5000);

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

    // ── Main render (called from WebSocket messages) ──
    let paused = false;
    function renderState(s) {
      if (paused) return;

      $("board-name").textContent = s.board_name;
      $("mcu-name").textContent   = s.mcu_name;
      $("tick-chip").textContent  = "tick=" + s.tick;
      $("i2c-chip").textContent   = "i2c ops=" + s.i2c.operation_count;

      $("temp-value").textContent  = fmt(s.climate.temperature_c,    " \u00B0C");
      $("hum-value").textContent   = fmt(s.climate.humidity_percent, " %");
      $("press-value").textContent = fmt(s.climate.pressure_pa,      " Pa");
      lcdLines[0].textContent = s.climate.physical_lcd_frame[0];
      lcdLines[1].textContent = s.climate.physical_lcd_frame[1];

      $("distance-value").textContent       = fmt(s.distance.distance_mm, " mm");
      $("distance-metric").textContent      = fmt(s.distance.distance_mm, " mm");
      $("distance-sensor-name").textContent = s.distance.sensor_name;
      $("servo-value").textContent          = s.servo.angle_degrees + " deg";

      $("accel-x").textContent = s.imu.accel_mg[0]  + " mg";
      $("accel-y").textContent = s.imu.accel_mg[1]  + " mg";
      $("accel-z").textContent = s.imu.accel_mg[2]  + " mg";
      $("gyro-x").textContent  = s.imu.gyro_mdps[0] + " mdps";
      $("gyro-y").textContent  = s.imu.gyro_mdps[1] + " mdps";
      $("gyro-z").textContent  = s.imu.gyro_mdps[2] + " mdps";

      $("motor-left").textContent  = s.motor_driver.left.direction  + " " + s.motor_driver.left.duty_percent  + "%";
      $("motor-right").textContent = s.motor_driver.right.direction + " " + s.motor_driver.right.duty_percent + "%";

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
      push("temp",   s.climate.temperature_c);
      push("hum",    s.climate.humidity_percent);
      push("press",  s.climate.pressure_pa);
      push("dist",   s.distance.distance_mm);
      push("accelz", s.imu.accel_mg[2]);
      sparkline("spark-temp",   hist.temp);
      sparkline("spark-hum",    hist.hum);
      sparkline("spark-press",  hist.press);
      sparkline("spark-dist",   hist.dist);
      sparkline("spark-accelz", hist.accelz);

      // visual simulation
      setLed(s.tick);
      setServo(s.servo.angle_degrees);
      setMotorViz("left",  s.motor_driver.left.direction,  s.motor_driver.left.duty_percent);
      setMotorViz("right", s.motor_driver.right.direction, s.motor_driver.right.duty_percent);
      setSonar(s.distance.distance_mm);
      setImuLevel(s.imu.accel_mg[0], s.imu.accel_mg[1]);

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

    // ── WebSocket with exponential-backoff reconnect ──
    let wsRetryDelay = 500;
    let wsRetryTimer = null;

    function connectWs() {
      clearTimeout(wsRetryTimer);
      const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
      const ws = new WebSocket(proto + '//' + location.host + '/api/ws');

      ws.onopen = () => {
        wsRetryDelay = 500;
        setOk();
      };

      ws.onmessage = (e) => {
        if (paused) return;
        const now = Date.now();
        if (now - lastRenderMs < renderIntervalMs) return;
        lastRenderMs = now;
        try { renderState(JSON.parse(e.data)); }
        catch(err) { setErr(err.message); }
      };

      ws.onclose = ws.onerror = () => {
        setErr('WebSocket reconnecting\u2026');
        wsRetryTimer = setTimeout(connectWs, wsRetryDelay);
        wsRetryDelay = Math.min(wsRetryDelay * 2, 10000);
      };
    }

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
    connectWs();
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
        // The dashboard uses WebSocket (/api/ws); /api/state remains as HTTP fallback.
        assert!(html.contains("/api/ws"));
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
        // E2E test runner
        assert!(html.contains("/api/test/stream"));
        assert!(html.contains("run-tests-btn"));
        assert!(html.contains("runTests"));
        assert!(html.contains("test-output"));
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
                diagram_lines: vec![],
            },
            i2c: I2cPanelState {
                operation_count: 12,
                recent_operations: vec!["WRITE addr=0x27".to_string()],
            },
        });

        assert!(json.contains("\"board_name\":\"Arduino Nano\""));
        assert!(json.contains("\"sensor_name\":\"HC-SR04\""));
        assert!(json.contains("\"operation_count\":12"));
    }
}
