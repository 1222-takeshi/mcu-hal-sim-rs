//! Browser dashboard state and HTML renderer.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
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
    pub oled: OledPanelState,
    pub diagnostics: DiagnosticsPanelState,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClimatePanelState {
    pub temperature_c: Option<f32>,
    pub humidity_percent: Option<f32>,
    pub pressure_pa: Option<u32>,
    pub app_frame: [String; 2],
    pub physical_lcd_frame: [String; 2],
}

#[derive(Debug, Clone, Serialize)]
pub struct DistancePanelState {
    pub distance_mm: Option<u32>,
    pub sensor_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImuPanelState {
    pub sensor_name: String,
    pub accel_mg: [i16; 3],
    pub gyro_mdps: [i32; 3],
    pub temperature_c: Option<f32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServoPanelState {
    pub angle_degrees: u16,
}

#[derive(Debug, Clone, Serialize)]
pub struct MotorChannelState {
    pub direction: String,
    pub duty_percent: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct MotorDriverPanelState {
    pub driver_name: String,
    pub left: MotorChannelState,
    pub right: MotorChannelState,
}

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
pub struct I2cPanelState {
    pub operation_count: usize,
    pub recent_operations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LightPanelState {
    pub lux_x100: u32,
    pub sensor_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CameraPanelState {
    pub width: u32,
    pub height: u32,
    pub sequence: u32,
    pub sensor_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GasPanelState {
    pub co2_ppm: Option<u16>,
    pub voc_ppb: Option<u16>,
    pub sensor_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RtcPanelState {
    pub datetime_str: String,
    pub sensor_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TofPanelState {
    pub distance_mm: Option<u32>,
    pub sensor_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OledPanelState {
    pub frame: [String; 2],
    pub sensor_name: String,
}

/// Diagnostics ring buffer state surfaced per tick.
///
/// A single diagnostics event with elapsed time, severity, and message.
#[derive(Debug, Clone, Default, Serialize)]
pub struct DiagEvent {
    /// Milliseconds elapsed since simulator start.
    #[serde(rename = "ts")]
    pub elapsed_ms: u64,
    /// Severity level: "info", "warn", or "error".
    #[serde(rename = "sev")]
    pub severity: String,
    /// Human-readable event description.
    #[serde(rename = "msg")]
    pub message: String,
}

/// `recent_events` holds up to 20 entries most-recent-first.
/// `event_count` is a monotonically increasing counter of all events ever
/// logged (useful for detecting new activity without diffing the list).
#[derive(Debug, Clone, Default, Serialize)]
pub struct DiagnosticsPanelState {
    pub recent_events: Vec<DiagEvent>,
    pub event_count: u32,
}

pub fn state_to_json(state: &DeviceDashboardState) -> String {
    serde_json::to_string(state).unwrap_or_default()
}

mod html;
pub use html::dashboard_html;
