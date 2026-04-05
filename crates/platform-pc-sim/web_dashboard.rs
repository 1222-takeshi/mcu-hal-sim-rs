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
            _ => output.push(character),
        }
    }
    output.push('"');
    output
}

pub fn dashboard_html() -> &'static str {
    r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Device Dashboard</title>
  <style>
    :root {
      --bg: #f6f1e7;
      --paper: rgba(255, 252, 247, 0.88);
      --ink: #1c2b28;
      --muted: #6c756f;
      --accent: #0f7c6b;
      --accent-2: #d95f3d;
      --line: rgba(28, 43, 40, 0.12);
      --shadow: 0 24px 80px rgba(33, 41, 38, 0.12);
    }
    * { box-sizing: border-box; }
    body {
      margin: 0;
      font-family: "IBM Plex Sans", "Avenir Next", sans-serif;
      color: var(--ink);
      background:
        radial-gradient(circle at top left, rgba(15,124,107,0.18), transparent 28rem),
        radial-gradient(circle at top right, rgba(217,95,61,0.12), transparent 24rem),
        linear-gradient(180deg, #faf5eb 0%, var(--bg) 100%);
      min-height: 100vh;
    }
    .shell {
      width: min(1220px, calc(100vw - 32px));
      margin: 24px auto 40px;
    }
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
    .hero-main {
      padding: 24px 24px 18px;
      position: relative;
      overflow: hidden;
    }
    .hero-main::after {
      content: "";
      position: absolute;
      inset: auto -12% -40% auto;
      width: 220px;
      height: 220px;
      background: radial-gradient(circle, rgba(15,124,107,0.18), transparent 65%);
      pointer-events: none;
    }
    .kicker {
      font-size: 12px;
      letter-spacing: 0.18em;
      text-transform: uppercase;
      color: var(--accent);
      margin-bottom: 10px;
    }
    .hero-title {
      font-family: "Iowan Old Style", "Palatino Linotype", serif;
      font-size: clamp(34px, 5vw, 58px);
      line-height: 0.95;
      margin: 0 0 10px;
    }
    .hero-sub {
      color: var(--muted);
      max-width: 42rem;
      line-height: 1.5;
    }
    .hero-meta {
      display: flex;
      gap: 10px;
      flex-wrap: wrap;
      margin-top: 18px;
    }
    .chip {
      padding: 8px 12px;
      border-radius: 999px;
      background: rgba(15,124,107,0.08);
      color: var(--ink);
      font-size: 13px;
    }
    .hero-side {
      padding: 20px;
      display: grid;
      align-content: space-between;
    }
    .hero-side .label {
      color: var(--muted);
      font-size: 13px;
      margin-bottom: 6px;
    }
    .hero-side .value {
      font-family: "Iowan Old Style", serif;
      font-size: 36px;
      margin-bottom: 14px;
    }
    .grid {
      display: grid;
      grid-template-columns: repeat(12, 1fr);
      gap: 16px;
    }
    .card {
      padding: 18px;
      min-height: 180px;
    }
    .span-4 { grid-column: span 4; }
    .span-6 { grid-column: span 6; }
    .span-8 { grid-column: span 8; }
    .span-12 { grid-column: span 12; }
    h2 {
      margin: 0 0 12px;
      font-size: 18px;
    }
    .metric-row {
      display: grid;
      grid-template-columns: repeat(3, 1fr);
      gap: 10px;
    }
    .metric {
      padding: 12px;
      border-radius: 16px;
      background: rgba(28,43,40,0.04);
    }
    .metric .name {
      color: var(--muted);
      font-size: 12px;
      margin-bottom: 6px;
      text-transform: uppercase;
      letter-spacing: 0.08em;
    }
    .metric .value {
      font-size: 24px;
      font-weight: 700;
    }
    .lcd {
      display: inline-block;
      padding: 16px 18px;
      border-radius: 18px;
      background: linear-gradient(180deg, #9bc39f 0%, #84b18a 100%);
      color: #112315;
      font-family: "IBM Plex Mono", "Menlo", monospace;
      font-size: 18px;
      box-shadow: inset 0 0 0 1px rgba(17,35,21,0.16);
    }
    .lcd-line { white-space: pre; }
    .wiring {
      font-family: "IBM Plex Mono", monospace;
      white-space: pre-wrap;
      line-height: 1.45;
      font-size: 14px;
    }
    .ops {
      margin: 0;
      padding: 0;
      list-style: none;
      font-family: "IBM Plex Mono", monospace;
      font-size: 13px;
    }
    .ops li {
      padding: 8px 0;
      border-bottom: 1px solid var(--line);
    }
    .axis {
      display: grid;
      grid-template-columns: repeat(3, 1fr);
      gap: 8px;
    }
    .axis div {
      padding: 10px;
      border-radius: 14px;
      background: rgba(217,95,61,0.07);
    }
    .motor {
      display: grid;
      grid-template-columns: repeat(2, 1fr);
      gap: 12px;
    }
    .footer {
      margin-top: 18px;
      color: var(--muted);
      font-size: 13px;
    }
    @media (max-width: 980px) {
      .hero, .grid { grid-template-columns: 1fr; }
      .span-4, .span-6, .span-8, .span-12 { grid-column: span 1; }
      .metric-row, .motor, .axis { grid-template-columns: 1fr; }
    }
  </style>
</head>
<body>
  <main class="shell">
    <section class="hero">
      <article class="panel hero-main">
        <div class="kicker">mcu-hal-sim-rs</div>
        <h1 class="hero-title" id="board-name">Device Dashboard</h1>
        <p class="hero-sub">
          Reference-path GUI for climate, distance, IMU, servo, and motor-driver simulation.
          The page polls the host simulator and renders both abstract app state and physical-device state.
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
          <div class="value" id="distance-value">-- mm</div>
          <div class="label">Servo</div>
          <div class="value" id="servo-value">-- deg</div>
        </div>
        <div class="label">Live polling from <code>/api/state</code></div>
      </aside>
    </section>

    <section class="grid">
      <article class="panel card span-6">
        <h2>Climate</h2>
        <div class="metric-row">
          <div class="metric"><div class="name">Temp</div><div class="value" id="temp-value">--</div></div>
          <div class="metric"><div class="name">Humidity</div><div class="value" id="hum-value">--</div></div>
          <div class="metric"><div class="name">Pressure</div><div class="value" id="press-value">--</div></div>
        </div>
      </article>
      <article class="panel card span-6">
        <h2>LCD</h2>
        <div class="lcd">
          <div class="lcd-line" id="lcd-line-1">                </div>
          <div class="lcd-line" id="lcd-line-2">                </div>
        </div>
        <div class="footer">Physical LCD frame decoded from backpack traffic.</div>
      </article>

      <article class="panel card span-4">
        <h2>HC-SR04</h2>
        <div class="metric">
          <div class="name" id="distance-sensor-name">Distance Sensor</div>
          <div class="value" id="distance-metric">-- mm</div>
        </div>
      </article>
      <article class="panel card span-4">
        <h2>MPU6050</h2>
        <div class="axis">
          <div><div class="name">Accel X</div><div class="value" id="accel-x">--</div></div>
          <div><div class="name">Accel Y</div><div class="value" id="accel-y">--</div></div>
          <div><div class="name">Accel Z</div><div class="value" id="accel-z">--</div></div>
          <div><div class="name">Gyro X</div><div class="value" id="gyro-x">--</div></div>
          <div><div class="name">Gyro Y</div><div class="value" id="gyro-y">--</div></div>
          <div><div class="name">Gyro Z</div><div class="value" id="gyro-z">--</div></div>
        </div>
      </article>
      <article class="panel card span-4">
        <h2>Motor Driver</h2>
        <div class="motor">
          <div class="metric">
            <div class="name">Left Channel</div>
            <div class="value" id="motor-left">--</div>
          </div>
          <div class="metric">
            <div class="name">Right Channel</div>
            <div class="value" id="motor-right">--</div>
          </div>
        </div>
      </article>

      <article class="panel card span-4">
        <h2>Wiring</h2>
        <div class="wiring" id="wiring-view"></div>
      </article>
      <article class="panel card span-8">
        <h2>I2C Activity</h2>
        <ul class="ops" id="i2c-ops"></ul>
      </article>
    </section>
  </main>

  <script>
    const fmt = (value, suffix = "") => value == null ? "--" : `${value}${suffix}`;
    const lcdLines = ["lcd-line-1", "lcd-line-2"].map(id => document.getElementById(id));
    async function refresh() {
      const response = await fetch("/api/state");
      const state = await response.json();

      document.getElementById("board-name").textContent = state.board_name;
      document.getElementById("mcu-name").textContent = state.mcu_name;
      document.getElementById("tick-chip").textContent = `tick=${state.tick}`;
      document.getElementById("i2c-chip").textContent = `i2c ops=${state.i2c.operation_count}`;

      document.getElementById("temp-value").textContent = fmt(state.climate.temperature_c, " C");
      document.getElementById("hum-value").textContent = fmt(state.climate.humidity_percent, " %");
      document.getElementById("press-value").textContent = fmt(state.climate.pressure_pa, " Pa");

      lcdLines[0].textContent = state.climate.physical_lcd_frame[0];
      lcdLines[1].textContent = state.climate.physical_lcd_frame[1];

      document.getElementById("distance-value").textContent = fmt(state.distance.distance_mm, " mm");
      document.getElementById("distance-metric").textContent = fmt(state.distance.distance_mm, " mm");
      document.getElementById("distance-sensor-name").textContent = state.distance.sensor_name;
      document.getElementById("servo-value").textContent = `${state.servo.angle_degrees} deg`;

      document.getElementById("accel-x").textContent = `${state.imu.accel_mg[0]} mg`;
      document.getElementById("accel-y").textContent = `${state.imu.accel_mg[1]} mg`;
      document.getElementById("accel-z").textContent = `${state.imu.accel_mg[2]} mg`;
      document.getElementById("gyro-x").textContent = `${state.imu.gyro_mdps[0]} mdps`;
      document.getElementById("gyro-y").textContent = `${state.imu.gyro_mdps[1]} mdps`;
      document.getElementById("gyro-z").textContent = `${state.imu.gyro_mdps[2]} mdps`;

      document.getElementById("motor-left").textContent =
        `${state.motor_driver.left.direction} ${state.motor_driver.left.duty_percent}%`;
      document.getElementById("motor-right").textContent =
        `${state.motor_driver.right.direction} ${state.motor_driver.right.duty_percent}%`;

      document.getElementById("wiring-view").textContent =
        state.wiring.diagram_lines.join("\n");

      const ops = document.getElementById("i2c-ops");
      ops.innerHTML = "";
      for (const line of state.i2c.recent_operations) {
        const li = document.createElement("li");
        li.textContent = line;
        ops.appendChild(li);
      }
    }

    refresh();
    setInterval(refresh, 500);
  </script>
</body>
</html>
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_contains_api_endpoint() {
        let html = dashboard_html();
        assert!(html.contains("/api/state"));
        assert!(html.contains("Device Dashboard"));
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
