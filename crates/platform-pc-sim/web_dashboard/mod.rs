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

mod html;
pub use html::dashboard_html;
