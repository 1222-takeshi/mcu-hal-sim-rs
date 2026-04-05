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
}

impl DeviceKind {
    pub fn label(&self) -> &str {
        match self {
            DeviceKind::Bme280 => "BME280",
            DeviceKind::Mpu6050 => "MPU6050",
            DeviceKind::Lcd1602 => "LCD1602",
            DeviceKind::HcSr04 => "HC-SR04",
        }
    }

    pub fn connection_type(&self) -> ConnectionType {
        match self {
            DeviceKind::HcSr04 => ConnectionType::Gpio,
            _ => ConnectionType::I2c,
        }
    }
}

/// How a device is connected to the board.
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionType {
    I2c,
    Gpio,
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
    pub servo_pin: String,
    pub devices: Vec<DeviceSpec>,
}

impl WiringConfig {
    /// Build the standard wiring config for a board profile.
    pub fn from_board(board: BoardProfile) -> Self {
        let devices = vec![
            DeviceSpec::i2c(DeviceKind::Bme280, 0x77),
            DeviceSpec::i2c(DeviceKind::Mpu6050, 0x68),
            DeviceSpec::i2c(DeviceKind::Lcd1602, 0x27),
            DeviceSpec::gpio(DeviceKind::HcSr04),
        ];
        Self {
            sda_pin: board.sda_pin().to_string(),
            scl_pin: board.scl_pin().to_string(),
            power_pin: board.power_pin().to_string(),
            ground_pin: "GND".to_string(),
            trig_pin: board.trig_pin().to_string(),
            echo_pin: board.echo_pin().to_string(),
            servo_pin: board.servo_pwm_pin().to_string(),
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
                };
                match d.address {
                    Some(a) => format!(
                        r#"{{"kind":"{kind}","address":"0x{a:02X}","label":"{}"}}"#,
                        d.label
                    ),
                    None => format!(r#"{{"kind":"{kind}","label":"{}"}}"#, d.label),
                }
            })
            .collect();
        format!(
            concat!(
                r#"{{"board":"{board}","sda_pin":"{sda}","scl_pin":"{scl}","#,
                r#""power_pin":"{vcc}","ground_pin":"{gnd}","#,
                r#""trig_pin":"{trig}","echo_pin":"{echo}","servo_pin":"{sv}","#,
                r#""devices":[{devs}]}}"#
            ),
            board = board_str,
            sda = self.sda_pin,
            scl = self.scl_pin,
            vcc = self.power_pin,
            gnd = self.ground_pin,
            trig = self.trig_pin,
            echo = self.echo_pin,
            sv = self.servo_pin,
            devs = devices.join(","),
        )
    }
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
        assert_eq!(cfg.servo_pin, "GPIO13");
    }

    #[test]
    fn wiring_config_nano_has_expected_pins() {
        let cfg = WiringConfig::from_board(BoardProfile::ArduinoNano);
        assert_eq!(cfg.sda_pin, "A4");
        assert_eq!(cfg.scl_pin, "A5");
        assert_eq!(cfg.power_pin, "5V");
    }

    #[test]
    fn wiring_config_has_four_devices() {
        let cfg = WiringConfig::from_board(BoardProfile::OriginalEsp32);
        assert_eq!(cfg.devices.len(), 4);
        assert_eq!(cfg.devices[0].kind, DeviceKind::Bme280);
        assert_eq!(cfg.devices[3].kind, DeviceKind::HcSr04);
        assert_eq!(cfg.devices[3].kind.connection_type(), ConnectionType::Gpio);
    }

    #[test]
    fn wiring_config_to_json_contains_board_and_devices() {
        let json = WiringConfig::from_board(BoardProfile::OriginalEsp32).to_json();
        assert!(json.contains(r#""board":"esp32""#));
        assert!(json.contains(r#""sda_pin":"GPIO21""#));
        assert!(json.contains(r#""address":"0x77""#));
        assert!(json.contains(r#""kind":"hc_sr04""#));
    }
}
