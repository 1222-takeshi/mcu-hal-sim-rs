//! Wiring configuration: which devices are connected to which board pins.
//!
//! Provides a board-independent description of the hardware setup used by
//! the visual wiring diagram generator.

use crate::dashboard::BoardProfile;

/// Physical device type attached to the board.
#[derive(Debug, Clone, PartialEq)]
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
    /// Build the standard wiring config for a board profile.
    ///
    /// Returns the full simulator configuration matching `DeviceSimulationRig`:
    /// BME280 (0x77), MPU6050 (0x68), LCD1602 (0x27), BH1750 (0x23),
    /// DS3231 (0x68; sim uses 0x69 to avoid MPU6050 collision), SGP30 (0x58),
    /// VL53L0X (0x29) on I²C; Servo and L298N on PWM; HC-SR04 and ESP32-CAM on GPIO.
    pub fn from_board(board: BoardProfile) -> Self {
        let devices = vec![
            DeviceSpec::i2c(DeviceKind::Bme280, 0x77),
            DeviceSpec::i2c(DeviceKind::Mpu6050, 0x68),
            DeviceSpec::i2c(DeviceKind::Lcd1602, 0x27),
            DeviceSpec::i2c(DeviceKind::Bh1750, 0x23),
            DeviceSpec::i2c(DeviceKind::Ds3231, 0x68),
            DeviceSpec::i2c(DeviceKind::Sgp30, 0x58),
            DeviceSpec::i2c(DeviceKind::Vl53l0x, 0x29),
            DeviceSpec::pwm(DeviceKind::Servo),
            DeviceSpec::pwm(DeviceKind::L298n),
            DeviceSpec::gpio(DeviceKind::HcSr04),
            DeviceSpec::gpio(DeviceKind::Esp32Cam),
        ];
        Self {
            sda_pin: board.sda_pin().to_string(),
            scl_pin: board.scl_pin().to_string(),
            power_pin: board.power_pin().to_string(),
            ground_pin: "GND".to_string(),
            trig_pin: board.trig_pin().to_string(),
            echo_pin: board.echo_pin().to_string(),
            cam_pin: board.cam_pin().to_string(),
            servo_pin: board.servo_pwm_pin().to_string(),
            motor_pin: board.motor_ena_pin().to_string(),
            board,
            devices,
        }
    }

    /// Serialise to a simple JSON string.
    pub fn to_json(&self) -> String {
        let board_str = match self.board {
            BoardProfile::OriginalEsp32 => "esp32",
            BoardProfile::ArduinoNano => "nano",
        };
        let devices: Vec<String> = self
            .devices
            .iter()
            .map(|d| {
                let kind = match d.kind {
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
                };
                match d.address {
                    Some(a) => format!(
                        r#"{{"kind":"{kind}","address":"0x{a:02X}","label":"{}"}}"#,
                        json_escape(&d.label)
                    ),
                    None => format!(r#"{{"kind":"{kind}","label":"{}"}}"#, json_escape(&d.label)),
                }
            })
            .collect();
        format!(
            concat!(
                r#"{{"board":"{board}","sda_pin":"{sda}","scl_pin":"{scl}","#,
                r#""power_pin":"{vcc}","ground_pin":"{gnd}","#,
                r#""trig_pin":"{trig}","echo_pin":"{echo}","cam_pin":"{cam}","#,
                r#""servo_pin":"{sv}","motor_pin":"{mot}","#,
                r#""devices":[{devs}]}}"#
            ),
            board = board_str,
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
}
