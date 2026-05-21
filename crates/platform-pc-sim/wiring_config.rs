//! Wiring configuration: which devices are connected to which board pins.
//!
//! Provides a board-independent description of the hardware setup used by
//! the visual wiring diagram generator.
//!
//! ## Board vs Sensor separation
//!
//! - [`crate::dashboard::BoardProfile`] describes **hardware** (pin assignments, MCU type).
//! - [`SensorProfile`] describes **which sensors** are attached.
//!
//! Use [`WiringConfig::from_board_with_sensors`] to combine the two.  
//! [`WiringConfig::from_board`] is a shortcut for [`SensorProfile::Full`].

use crate::dashboard::BoardProfile;

/// Which set of sensors/actuators is attached to the board.
///
/// Each profile maps to a fixed subset of [`DeviceKind`]s included in the
/// [`WiringConfig`].  Add new profiles here to describe additional hardware
/// configurations without changing any other code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SensorProfile {
    /// All 11 devices (BME280, MPU6050, LCD1602, BH1750, DS3231, SGP30,
    /// VL53L0X, Servo, L298N, HC-SR04, ESP32-CAM).  This is the default.
    #[default]
    Full,
    /// Climate station: BME280 + BH1750 + SGP30 + DS3231 + LCD1602.
    ClimateStation,
    /// Robot base: MPU6050 + HC-SR04 + VL53L0X + Servo + L298N.
    RobotBase,
    /// Minimal starter: BME280 + LCD1602.
    Minimal,
}

impl SensorProfile {
    /// Parse a sensor profile from a URL-friendly slug string.
    ///
    /// ```
    /// use platform_pc_sim::wiring_config::SensorProfile;
    /// assert_eq!(SensorProfile::from_slug("climate"), Some(SensorProfile::ClimateStation));
    /// assert_eq!(SensorProfile::from_slug("unknown"), None);
    /// ```
    pub fn from_slug(s: &str) -> Option<Self> {
        match s {
            "full" => Some(Self::Full),
            "climate" => Some(Self::ClimateStation),
            "robot" => Some(Self::RobotBase),
            "minimal" => Some(Self::Minimal),
            _ => None,
        }
    }

    /// URL-friendly slug for this profile.
    ///
    /// ```
    /// use platform_pc_sim::wiring_config::SensorProfile;
    /// assert_eq!(SensorProfile::Full.slug(), "full");
    /// assert_eq!(SensorProfile::ClimateStation.slug(), "climate");
    /// ```
    pub fn slug(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::ClimateStation => "climate",
            Self::RobotBase => "robot",
            Self::Minimal => "minimal",
        }
    }

    /// Human-readable display name.
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Full => "Full (all devices)",
            Self::ClimateStation => "Climate Station",
            Self::RobotBase => "Robot Base",
            Self::Minimal => "Minimal (BME280 + LCD)",
        }
    }

    /// Returns all sensor profile variants.
    ///
    /// ```
    /// use platform_pc_sim::wiring_config::SensorProfile;
    /// assert_eq!(SensorProfile::all_variants().len(), 4);
    /// assert_eq!(SensorProfile::all_variants()[0], SensorProfile::Full);
    /// ```
    pub fn all_variants() -> &'static [SensorProfile] {
        &[
            Self::Full,
            Self::ClimateStation,
            Self::RobotBase,
            Self::Minimal,
        ]
    }

    /// Returns all available sensor profiles as `(slug, display_name)` pairs.
    ///
    /// ```
    /// use platform_pc_sim::wiring_config::SensorProfile;
    /// let profiles = SensorProfile::all();
    /// assert!(profiles.iter().any(|(slug, _)| *slug == "full"));
    /// assert!(profiles.iter().any(|(slug, _)| *slug == "climate"));
    /// ```
    pub fn all() -> Vec<(&'static str, &'static str)> {
        Self::all_variants()
            .iter()
            .map(|p| (p.slug(), p.display_name()))
            .collect()
    }

    /// Device kinds included in this profile, in canonical dashboard order.
    pub fn device_kinds(self) -> Vec<DeviceKind> {
        match self {
            Self::Full => vec![
                DeviceKind::Bme280,
                DeviceKind::Mpu6050,
                DeviceKind::Lcd1602,
                DeviceKind::Bh1750,
                DeviceKind::Ds3231,
                DeviceKind::Sgp30,
                DeviceKind::Vl53l0x,
                DeviceKind::Servo,
                DeviceKind::L298n,
                DeviceKind::HcSr04,
                DeviceKind::Esp32Cam,
            ],
            Self::ClimateStation => vec![
                DeviceKind::Bme280,
                DeviceKind::Bh1750,
                DeviceKind::Sgp30,
                DeviceKind::Ds3231,
                DeviceKind::Lcd1602,
            ],
            Self::RobotBase => vec![
                DeviceKind::Mpu6050,
                DeviceKind::Vl53l0x,
                DeviceKind::HcSr04,
                DeviceKind::Servo,
                DeviceKind::L298n,
            ],
            Self::Minimal => vec![DeviceKind::Bme280, DeviceKind::Lcd1602],
        }
    }
}

/// Physical device type attached to the board.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeviceKind {
    Bme280,
    Mpu6050,
    Lcd1602,
    HcSr04,
    Bh1750,
    Servo,
    L298n,
    Esp32Cam,
    /// DS3231 high-precision RTC (I2C 0x68)
    Ds3231,
    /// SGP30 CO₂/VOC gas sensor (I2C 0x58)
    Sgp30,
    /// VL53L0X ToF distance sensor (I2C 0x29)
    Vl53l0x,
}

impl DeviceKind {
    pub fn slug(self) -> &'static str {
        match self {
            DeviceKind::Bme280 => "bme280",
            DeviceKind::Mpu6050 => "mpu6050",
            DeviceKind::Lcd1602 => "lcd1602",
            DeviceKind::HcSr04 => "hc_sr04",
            DeviceKind::Bh1750 => "bh1750",
            DeviceKind::Servo => "servo",
            DeviceKind::L298n => "l298n",
            DeviceKind::Esp32Cam => "esp32_cam",
            DeviceKind::Ds3231 => "ds3231",
            DeviceKind::Sgp30 => "sgp30",
            DeviceKind::Vl53l0x => "vl53l0x",
        }
    }

    pub fn from_slug(s: &str) -> Option<Self> {
        match s {
            "bme280" => Some(DeviceKind::Bme280),
            "mpu6050" => Some(DeviceKind::Mpu6050),
            "lcd1602" => Some(DeviceKind::Lcd1602),
            "hc_sr04" => Some(DeviceKind::HcSr04),
            "bh1750" => Some(DeviceKind::Bh1750),
            "servo" => Some(DeviceKind::Servo),
            "l298n" => Some(DeviceKind::L298n),
            "esp32_cam" => Some(DeviceKind::Esp32Cam),
            "ds3231" => Some(DeviceKind::Ds3231),
            "sgp30" => Some(DeviceKind::Sgp30),
            "vl53l0x" => Some(DeviceKind::Vl53l0x),
            _ => None,
        }
    }

    pub fn all() -> &'static [DeviceKind] {
        &[
            DeviceKind::Bme280,
            DeviceKind::Mpu6050,
            DeviceKind::Lcd1602,
            DeviceKind::Bh1750,
            DeviceKind::Ds3231,
            DeviceKind::Sgp30,
            DeviceKind::Vl53l0x,
            DeviceKind::Servo,
            DeviceKind::L298n,
            DeviceKind::HcSr04,
            DeviceKind::Esp32Cam,
        ]
    }

    pub fn label(&self) -> &str {
        match self {
            DeviceKind::Bme280 => "BME280",
            DeviceKind::Mpu6050 => "MPU6050",
            DeviceKind::Lcd1602 => "LCD1602",
            DeviceKind::HcSr04 => "HC-SR04",
            DeviceKind::Bh1750 => "BH1750",
            DeviceKind::Servo => "Servo",
            DeviceKind::L298n => "L298N",
            DeviceKind::Esp32Cam => "ESP32-CAM",
            DeviceKind::Ds3231 => "DS3231",
            DeviceKind::Sgp30 => "SGP30",
            DeviceKind::Vl53l0x => "VL53L0X",
        }
    }

    pub fn connection_type(&self) -> ConnectionType {
        match self {
            DeviceKind::HcSr04 | DeviceKind::Esp32Cam => ConnectionType::Gpio,
            DeviceKind::Servo | DeviceKind::L298n => ConnectionType::Pwm,
            _ => ConnectionType::I2c,
        }
    }

    fn default_address(self) -> Option<u8> {
        match self {
            DeviceKind::Bme280 => Some(0x77),
            DeviceKind::Mpu6050 => Some(0x68),
            DeviceKind::Lcd1602 => Some(0x27),
            DeviceKind::Bh1750 => Some(0x23),
            DeviceKind::Ds3231 => Some(0x68),
            DeviceKind::Sgp30 => Some(0x58),
            DeviceKind::Vl53l0x => Some(0x29),
            DeviceKind::HcSr04 | DeviceKind::Servo | DeviceKind::L298n | DeviceKind::Esp32Cam => {
                None
            }
        }
    }
}

/// How a device is connected to the board.
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionType {
    I2c,
    Gpio,
    /// PWM signal (Servo, motor driver enable pins).
    Pwm,
}

/// Specification of a single device in the wiring config.
#[derive(Debug, Clone)]
pub struct DeviceSpec {
    pub kind: DeviceKind,
    /// I2C address (None for GPIO devices).
    pub address: Option<u8>,
    pub label: String,
}

impl DeviceSpec {
    pub fn i2c(kind: DeviceKind, address: u8) -> Self {
        let label = format!("{} (0x{address:02X})", kind.label());
        Self {
            kind,
            address: Some(address),
            label,
        }
    }

    pub fn gpio(kind: DeviceKind) -> Self {
        Self::non_i2c(kind)
    }

    pub fn pwm(kind: DeviceKind) -> Self {
        Self::non_i2c(kind)
    }

    fn non_i2c(kind: DeviceKind) -> Self {
        let label = kind.label().to_string();
        Self {
            kind,
            address: None,
            label,
        }
    }
}

/// Full wiring description for the board + all attached devices.
#[derive(Debug, Clone)]
pub struct WiringConfig {
    pub board: BoardProfile,
    pub sensor_profile: SensorProfile,
    pub show_bus_labels: bool,
    pub sda_pin: String,
    pub scl_pin: String,
    pub power_pin: String,
    pub ground_pin: String,
    pub trig_pin: String,
    pub echo_pin: String,
    /// Representative pin for the camera module (GPIO0 / boot pin on ESP32).
    pub cam_pin: String,
    pub servo_pin: String,
    /// L298N motor driver enable-A pin (ENA).
    pub motor_pin: String,
    pub devices: Vec<DeviceSpec>,
}

impl WiringConfig {
    /// Build the wiring config for a board profile with a specific sensor set.
    ///
    /// # Examples
    ///
    /// ```
    /// use platform_pc_sim::dashboard::BoardProfile;
    /// use platform_pc_sim::wiring_config::{SensorProfile, WiringConfig};
    ///
    /// let cfg = WiringConfig::from_board_with_sensors(
    ///     BoardProfile::OriginalEsp32,
    ///     SensorProfile::Minimal,
    /// );
    /// assert_eq!(cfg.devices.len(), 2);
    /// ```
    pub fn from_board_with_sensors(board: BoardProfile, sensor_profile: SensorProfile) -> Self {
        Self::from_board_with_selected_devices(
            board,
            sensor_profile,
            &sensor_profile.device_kinds(),
        )
    }

    pub fn from_board_with_selected_devices(
        board: BoardProfile,
        sensor_profile: SensorProfile,
        selected_devices: &[DeviceKind],
    ) -> Self {
        Self {
            sda_pin: board.sda_pin().to_string(),
            scl_pin: board.scl_pin().to_string(),
            show_bus_labels: false,
            power_pin: board.power_pin().to_string(),
            ground_pin: "GND".to_string(),
            trig_pin: board.trig_pin().to_string(),
            echo_pin: board.echo_pin().to_string(),
            cam_pin: board.cam_pin().to_string(),
            servo_pin: board.servo_pwm_pin().to_string(),
            motor_pin: board.motor_ena_pin().to_string(),
            devices: normalize_device_selection(selected_devices)
                .into_iter()
                .map(device_spec_from_kind)
                .collect(),
            board,
            sensor_profile,
        }
    }

    pub fn with_bus_labels(mut self, show_bus_labels: bool) -> Self {
        self.show_bus_labels = show_bus_labels;
        self
    }

    /// Build the standard wiring config for a board profile with all devices.
    ///
    /// Equivalent to `from_board_with_sensors(board, SensorProfile::Full)`.
    ///
    /// Returns the full simulator configuration matching `DeviceSimulationRig`:
    /// BME280 (0x77), MPU6050 (0x68), LCD1602 (0x27), BH1750 (0x23),
    /// DS3231 (0x68), VL53L0X (0x29) on I2C; Servo and L298N on PWM; HC-SR04
    /// and ESP32-CAM on GPIO.
    ///
    /// The simulator attaches DS3231 internally at `0x69` to avoid colliding
    /// with MPU6050 on the virtual bus, but dashboard-facing wiring/state
    /// surfaces translate it back to the logical hardware address `0x68`.
    pub fn from_board(board: BoardProfile) -> Self {
        Self::from_board_with_sensors(board, SensorProfile::Full)
    }

    /// Serialise to a simple JSON string.
    pub fn to_json(&self) -> String {
        let board_str = match self.board {
            BoardProfile::OriginalEsp32 => "esp32",
            BoardProfile::ArduinoNano => "nano",
        };
        let selected_devices: Vec<String> = self
            .devices
            .iter()
            .map(|device| format!(r#""{}""#, device.kind.slug()))
            .collect();
        let available_devices: Vec<String> = DeviceKind::all()
            .iter()
            .map(|kind| {
                format!(
                    r#"{{"kind":"{}","label":"{}","enabled":{}}}"#,
                    kind.slug(),
                    json_escape(kind.label()),
                    self.devices.iter().any(|device| device.kind == *kind)
                )
            })
            .collect();
        let devices: Vec<String> = self
            .devices
            .iter()
            .map(|d| match d.address {
                Some(a) => format!(
                    r#"{{"kind":"{}","address":"0x{a:02X}","label":"{}"}}"#,
                    d.kind.slug(),
                    json_escape(&d.label)
                ),
                None => format!(
                    r#"{{"kind":"{}","label":"{}"}}"#,
                    d.kind.slug(),
                    json_escape(&d.label)
                ),
            })
            .collect();
        format!(
            concat!(
                r#"{{"board":"{board}","sensor_profile":"{sp}","#,
                r#""show_bus_labels":{show_bus_labels},"#,
                r#""selected_devices":[{selected}],"available_devices":[{available}],"#,
                r#""sda_pin":"{sda}","scl_pin":"{scl}","#,
                r#""power_pin":"{vcc}","ground_pin":"{gnd}","#,
                r#""trig_pin":"{trig}","echo_pin":"{echo}","cam_pin":"{cam}","#,
                r#""servo_pin":"{sv}","motor_pin":"{mot}","#,
                r#""devices":[{devs}]}}"#
            ),
            board = board_str,
            sp = self.sensor_profile.slug(),
            show_bus_labels = self.show_bus_labels,
            selected = selected_devices.join(","),
            available = available_devices.join(","),
            sda = json_escape(&self.sda_pin),
            scl = json_escape(&self.scl_pin),
            vcc = json_escape(&self.power_pin),
            gnd = json_escape(&self.ground_pin),
            trig = json_escape(&self.trig_pin),
            echo = json_escape(&self.echo_pin),
            cam = json_escape(&self.cam_pin),
            sv = json_escape(&self.servo_pin),
            mot = json_escape(&self.motor_pin),
            devs = devices.join(","),
        )
    }
}

fn device_spec_from_kind(kind: DeviceKind) -> DeviceSpec {
    match kind.connection_type() {
        ConnectionType::I2c => DeviceSpec::i2c(
            kind,
            kind.default_address().expect("i2c devices have an address"),
        ),
        ConnectionType::Gpio => DeviceSpec::gpio(kind),
        ConnectionType::Pwm => DeviceSpec::pwm(kind),
    }
}

fn normalize_device_selection(selected_devices: &[DeviceKind]) -> Vec<DeviceKind> {
    DeviceKind::all()
        .iter()
        .copied()
        .filter(|kind| selected_devices.contains(kind))
        .collect()
}

/// Escape a string for embedding in a JSON value (no surrounding quotes added).
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04X}", c as u32));
            }
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wiring_config_esp32_has_expected_pins() {
        let cfg = WiringConfig::from_board(BoardProfile::OriginalEsp32);
        assert_eq!(cfg.sda_pin, "GPIO21");
        assert_eq!(cfg.scl_pin, "GPIO22");
        assert_eq!(cfg.power_pin, "3V3");
        assert_eq!(cfg.trig_pin, "GPIO5");
        assert_eq!(cfg.echo_pin, "GPIO18");
        assert_eq!(cfg.servo_pin, "GPIO13");
        assert_eq!(cfg.motor_pin, "GPIO25");
        assert_eq!(cfg.cam_pin, "GPIO0");
    }

    #[test]
    fn wiring_config_nano_has_expected_pins() {
        let cfg = WiringConfig::from_board(BoardProfile::ArduinoNano);
        assert_eq!(cfg.sda_pin, "A4");
        assert_eq!(cfg.scl_pin, "A5");
        assert_eq!(cfg.power_pin, "5V");
        assert_eq!(cfg.trig_pin, "D2");
        assert_eq!(cfg.echo_pin, "D3");
        assert_eq!(cfg.servo_pin, "D9");
        assert_eq!(cfg.motor_pin, "D5");
        assert_eq!(cfg.cam_pin, "N/A");
    }

    #[test]
    fn wiring_config_has_eleven_devices() {
        let cfg = WiringConfig::from_board(BoardProfile::OriginalEsp32);
        assert_eq!(cfg.devices.len(), 11);
        // I2C devices: [0-6]
        assert_eq!(cfg.devices[0].kind, DeviceKind::Bme280);
        assert_eq!(cfg.devices[3].kind, DeviceKind::Bh1750);
        assert_eq!(cfg.devices[4].kind, DeviceKind::Ds3231);
        assert_eq!(cfg.devices[5].kind, DeviceKind::Sgp30);
        assert_eq!(cfg.devices[6].kind, DeviceKind::Vl53l0x);
        // PWM: [7-8]
        assert_eq!(cfg.devices[7].kind, DeviceKind::Servo);
        assert_eq!(cfg.devices[7].kind.connection_type(), ConnectionType::Pwm);
        assert_eq!(cfg.devices[8].kind, DeviceKind::L298n);
        assert_eq!(cfg.devices[8].kind.connection_type(), ConnectionType::Pwm);
        // GPIO: [9-10]
        assert_eq!(cfg.devices[9].kind, DeviceKind::HcSr04);
        assert_eq!(cfg.devices[9].kind.connection_type(), ConnectionType::Gpio);
        assert_eq!(cfg.devices[10].kind, DeviceKind::Esp32Cam);
    }

    #[test]
    fn wiring_config_devices_have_correct_connection_types() {
        let cfg = WiringConfig::from_board(BoardProfile::OriginalEsp32);
        let expected = [
            ConnectionType::I2c,  // [0] BME280
            ConnectionType::I2c,  // [1] MPU6050
            ConnectionType::I2c,  // [2] LCD1602
            ConnectionType::I2c,  // [3] BH1750
            ConnectionType::I2c,  // [4] DS3231
            ConnectionType::I2c,  // [5] SGP30
            ConnectionType::I2c,  // [6] VL53L0X
            ConnectionType::Pwm,  // [7] Servo
            ConnectionType::Pwm,  // [8] L298N
            ConnectionType::Gpio, // [9] HC-SR04
            ConnectionType::Gpio, // [10] ESP32-CAM
        ];
        for (i, exp) in expected.iter().enumerate() {
            assert_eq!(
                cfg.devices[i].kind.connection_type(),
                *exp,
                "device[{i}] has wrong connection type"
            );
        }
    }

    #[test]
    fn wiring_config_to_json_contains_board_and_devices() {
        let json = WiringConfig::from_board(BoardProfile::OriginalEsp32).to_json();
        assert!(json.contains(r#""board":"esp32""#));
        assert!(json.contains(r#""sensor_profile":"full""#));
        assert!(json.contains(r#""sda_pin":"GPIO21""#));
        assert!(json.contains(r#""address":"0x77""#));
        assert!(json.contains(r#""kind":"hc_sr04""#));
        assert!(json.contains(r#""kind":"bh1750""#));
        assert!(json.contains(r#""kind":"servo""#));
        assert!(json.contains(r#""kind":"l298n""#));
        assert!(json.contains(r#""kind":"esp32_cam""#));
        assert!(json.contains(r#""kind":"ds3231""#));
        assert!(json.contains(r#""kind":"sgp30""#));
        assert!(json.contains(r#""kind":"vl53l0x""#));
        assert!(json.contains(r#""cam_pin":"GPIO0""#));
        assert!(json.contains(r#""motor_pin":"GPIO25""#));
    }

    // ── SensorProfile tests ────────────────────────────────────────────────

    #[test]
    fn sensor_profile_minimal_has_two_devices() {
        let cfg = WiringConfig::from_board_with_sensors(
            BoardProfile::OriginalEsp32,
            SensorProfile::Minimal,
        );
        assert_eq!(cfg.devices.len(), 2);
        assert_eq!(cfg.devices[0].kind, DeviceKind::Bme280);
        assert_eq!(cfg.devices[1].kind, DeviceKind::Lcd1602);
        assert_eq!(cfg.sensor_profile, SensorProfile::Minimal);
    }

    #[test]
    fn sensor_profile_climate_station_has_five_devices() {
        let cfg = WiringConfig::from_board_with_sensors(
            BoardProfile::OriginalEsp32,
            SensorProfile::ClimateStation,
        );
        assert_eq!(cfg.devices.len(), 5);
        let kinds: Vec<_> = cfg.devices.iter().map(|d| &d.kind).collect();
        assert!(kinds.contains(&&DeviceKind::Bme280));
        assert!(kinds.contains(&&DeviceKind::Bh1750));
        assert!(kinds.contains(&&DeviceKind::Sgp30));
        assert!(kinds.contains(&&DeviceKind::Ds3231));
        assert!(kinds.contains(&&DeviceKind::Lcd1602));
    }

    #[test]
    fn sensor_profile_robot_base_has_five_devices() {
        let cfg = WiringConfig::from_board_with_sensors(
            BoardProfile::OriginalEsp32,
            SensorProfile::RobotBase,
        );
        assert_eq!(cfg.devices.len(), 5);
        let kinds: Vec<_> = cfg.devices.iter().map(|d| &d.kind).collect();
        assert!(kinds.contains(&&DeviceKind::Mpu6050));
        assert!(kinds.contains(&&DeviceKind::Vl53l0x));
        assert!(kinds.contains(&&DeviceKind::HcSr04));
        assert!(kinds.contains(&&DeviceKind::Servo));
        assert!(kinds.contains(&&DeviceKind::L298n));
    }

    #[test]
    fn sensor_profile_full_is_default_and_has_eleven_devices() {
        let full =
            WiringConfig::from_board_with_sensors(BoardProfile::OriginalEsp32, SensorProfile::Full);
        let default = WiringConfig::from_board(BoardProfile::OriginalEsp32);
        assert_eq!(full.devices.len(), 11);
        assert_eq!(default.devices.len(), 11);
        assert_eq!(full.sensor_profile, SensorProfile::Full);
        assert_eq!(default.sensor_profile, SensorProfile::Full);
    }

    #[test]
    fn sensor_profile_from_slug_roundtrips() {
        for (slug, _) in SensorProfile::all() {
            let parsed = SensorProfile::from_slug(slug);
            assert!(parsed.is_some(), "slug '{slug}' should parse");
            assert_eq!(parsed.unwrap().slug(), slug);
        }
    }

    #[test]
    fn sensor_profile_from_slug_rejects_unknown() {
        assert!(SensorProfile::from_slug("unknown-profile").is_none());
        assert!(SensorProfile::from_slug("").is_none());
    }

    #[test]
    fn device_kind_slug_roundtrips() {
        for kind in DeviceKind::all() {
            assert_eq!(DeviceKind::from_slug(kind.slug()), Some(*kind));
        }
    }

    #[test]
    fn sensor_profile_all_has_four_entries() {
        assert_eq!(SensorProfile::all().len(), 4);
    }

    #[test]
    fn sensor_profile_display_names_match_all_variants() {
        for p in SensorProfile::all_variants() {
            let name = p.display_name();
            assert!(
                !name.is_empty(),
                "display_name() must not be empty for {:?}",
                p
            );
            // slug roundtrip
            assert_eq!(SensorProfile::from_slug(p.slug()).unwrap(), *p);
        }
    }

    #[test]
    fn sensor_profile_default_is_full() {
        assert_eq!(SensorProfile::default(), SensorProfile::Full);
    }

    #[test]
    fn wiring_config_from_selected_devices_allows_custom_selection() {
        let cfg = WiringConfig::from_board_with_selected_devices(
            BoardProfile::OriginalEsp32,
            SensorProfile::Minimal,
            &[DeviceKind::Servo, DeviceKind::Bme280, DeviceKind::Servo],
        );
        let kinds: Vec<_> = cfg.devices.iter().map(|d| d.kind).collect();
        assert_eq!(kinds, vec![DeviceKind::Bme280, DeviceKind::Servo]);
        assert_eq!(cfg.sensor_profile, SensorProfile::Minimal);
    }

    #[test]
    fn wiring_config_uses_simulated_ds3231_address() {
        let cfg = WiringConfig::from_board_with_selected_devices(
            BoardProfile::OriginalEsp32,
            SensorProfile::ClimateStation,
            &[DeviceKind::Ds3231],
        );

        assert_eq!(cfg.devices.len(), 1);
        assert_eq!(cfg.devices[0].kind, DeviceKind::Ds3231);
        assert_eq!(cfg.devices[0].address, Some(0x68));
        assert_eq!(cfg.devices[0].label, "DS3231 (0x68)");
    }

    #[test]
    fn wiring_config_json_includes_sensor_profile_slug() {
        let cfg = WiringConfig::from_board_with_sensors(
            BoardProfile::OriginalEsp32,
            SensorProfile::ClimateStation,
        );
        let json = cfg.to_json();
        assert!(json.contains(r#""sensor_profile":"climate""#));
        assert!(
            json.contains(r#""selected_devices":["bme280","lcd1602","bh1750","ds3231","sgp30"]"#)
        );
        assert!(json.contains(r#""kind":"servo","label":"Servo","enabled":false"#));
        assert!(json.contains(r#""kind":"hc_sr04","label":"HC-SR04","enabled":false"#));
    }

    #[test]
    fn wiring_config_json_includes_selected_and_available_devices() {
        let cfg = WiringConfig::from_board_with_selected_devices(
            BoardProfile::OriginalEsp32,
            SensorProfile::Minimal,
            &[DeviceKind::Bme280, DeviceKind::Servo],
        );
        let json = cfg.to_json();
        assert!(json.contains(r#""selected_devices":["bme280","servo"]"#));
        assert!(json.contains(r#""available_devices":["#));
        assert!(json.contains(r#""kind":"bme280","label":"BME280","enabled":true"#));
        assert!(json.contains(r#""kind":"lcd1602","label":"LCD1602","enabled":false"#));
    }

    #[test]
    fn wiring_config_json_tracks_bus_label_visibility() {
        let compact_json = WiringConfig::from_board(BoardProfile::OriginalEsp32).to_json();
        assert!(compact_json.contains(r#""show_bus_labels":false"#));

        let detailed_json = WiringConfig::from_board(BoardProfile::OriginalEsp32)
            .with_bus_labels(true)
            .to_json();
        assert!(detailed_json.contains(r#""show_bus_labels":true"#));
    }

    #[test]
    fn wiring_config_pins_are_board_specific_regardless_of_sensor_profile() {
        let esp = WiringConfig::from_board_with_sensors(
            BoardProfile::OriginalEsp32,
            SensorProfile::Minimal,
        );
        let nano = WiringConfig::from_board_with_sensors(
            BoardProfile::ArduinoNano,
            SensorProfile::Minimal,
        );
        assert_eq!(esp.sda_pin, "GPIO21");
        assert_eq!(nano.sda_pin, "A4");
        // Both have the same sensor count despite different boards
        assert_eq!(esp.devices.len(), nano.devices.len());
    }
}
