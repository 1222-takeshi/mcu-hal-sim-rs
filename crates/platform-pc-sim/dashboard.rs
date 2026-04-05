//! Terminal dashboard rendering helpers.

use crate::bme280_mock::Bme280ControlRegisters;
use crate::virtual_i2c::VirtualI2cOperation;
use hal_api::display::TextFrame16x2;
use hal_api::sensor::EnvReading;
use std::fmt::Write as _;
use std::string::String;
use std::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardProfile {
    OriginalEsp32,
    ArduinoNano,
}

impl BoardProfile {
    pub fn from_arg(value: Option<&str>) -> Self {
        match value {
            Some("nano") | Some("arduino-nano") => Self::ArduinoNano,
            _ => Self::OriginalEsp32,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::OriginalEsp32 => "original ESP32",
            Self::ArduinoNano => "Arduino Nano",
        }
    }

    pub fn mcu(self) -> &'static str {
        match self {
            Self::OriginalEsp32 => "ESP32",
            Self::ArduinoNano => "ATmega328P",
        }
    }

    pub fn sda_pin(self) -> &'static str {
        match self {
            Self::OriginalEsp32 => "GPIO21",
            Self::ArduinoNano => "A4",
        }
    }

    pub fn scl_pin(self) -> &'static str {
        match self {
            Self::OriginalEsp32 => "GPIO22",
            Self::ArduinoNano => "A5",
        }
    }

    pub fn power_pin(self) -> &'static str {
        match self {
            Self::OriginalEsp32 => "3V3",
            Self::ArduinoNano => "5V",
        }
    }

    pub fn trig_pin(self) -> &'static str {
        match self {
            Self::OriginalEsp32 => "GPIO5",
            Self::ArduinoNano => "D2",
        }
    }

    pub fn echo_pin(self) -> &'static str {
        match self {
            Self::OriginalEsp32 => "GPIO18",
            Self::ArduinoNano => "D3",
        }
    }

    pub fn servo_pwm_pin(self) -> &'static str {
        match self {
            Self::OriginalEsp32 => "GPIO13",
            Self::ArduinoNano => "D9",
        }
    }

    pub fn motor_ena_pin(self) -> &'static str {
        match self {
            Self::OriginalEsp32 => "GPIO25",
            Self::ArduinoNano => "D5",
        }
    }

    pub fn motor_in1_pin(self) -> &'static str {
        match self {
            Self::OriginalEsp32 => "GPIO26",
            Self::ArduinoNano => "D6",
        }
    }

    pub fn motor_in2_pin(self) -> &'static str {
        match self {
            Self::OriginalEsp32 => "GPIO27",
            Self::ArduinoNano => "D7",
        }
    }

    pub fn motor_enb_pin(self) -> &'static str {
        match self {
            Self::OriginalEsp32 => "GPIO32",
            Self::ArduinoNano => "D10",
        }
    }

    pub fn motor_in3_pin(self) -> &'static str {
        match self {
            Self::OriginalEsp32 => "GPIO33",
            Self::ArduinoNano => "D8",
        }
    }

    pub fn motor_in4_pin(self) -> &'static str {
        match self {
            Self::OriginalEsp32 => "GPIO14",
            Self::ArduinoNano => "D11",
        }
    }
}

pub struct DashboardSnapshot<'a> {
    pub board: BoardProfile,
    pub tick: u32,
    pub refresh_period_ticks: u32,
    pub reading: Option<EnvReading>,
    pub rendered_frame: Option<TextFrame16x2>,
    pub physical_frame: TextFrame16x2,
    pub bme280_registers: Bme280ControlRegisters,
    pub bme280_raw_sample: [u8; 8],
    pub lcd_initialized: bool,
    pub lcd_backlight: bool,
    pub attached_addresses: &'a [u8],
    pub operations: &'a [VirtualI2cOperation],
}

pub fn render_dashboard(snapshot: &DashboardSnapshot<'_>) -> String {
    let mut output = String::new();
    let _ = write!(output, "\x1B[2J\x1B[H");
    let _ = writeln!(output, "=== Climate Dashboard Sim ===");
    let _ = writeln!(
        output,
        "Board: {} ({})  tick={}  refresh={} ticks",
        snapshot.board.name(),
        snapshot.board.mcu(),
        snapshot.tick,
        snapshot.refresh_period_ticks
    );
    let _ = writeln!(output);
    let _ = writeln!(output, "Wiring");
    let _ = writeln!(
        output,
        "  {} SDA -----+---- BME280 SDA",
        snapshot.board.sda_pin()
    );
    let _ = writeln!(output, "               +---- LCD1602 SDA");
    let _ = writeln!(
        output,
        "  {} SCL -----+---- BME280 SCL",
        snapshot.board.scl_pin()
    );
    let _ = writeln!(output, "               +---- LCD1602 SCL");
    let _ = writeln!(
        output,
        "  {} --------+---- BME280 VCC",
        snapshot.board.power_pin()
    );
    let _ = writeln!(output, "               +---- LCD1602 VCC");
    let _ = writeln!(output, "  GND --------+---- shared ground");
    let _ = writeln!(output);
    let _ = writeln!(
        output,
        "Attached I2C devices: {}",
        format_addresses(snapshot.attached_addresses)
    );
    let _ = writeln!(
        output,
        "BME280 ctrl_hum=0x{:02X} ctrl_meas=0x{:02X} config=0x{:02X}",
        snapshot.bme280_registers.ctrl_hum,
        snapshot.bme280_registers.ctrl_meas,
        snapshot.bme280_registers.config
    );
    let _ = writeln!(
        output,
        "LCD1602 initialized={} backlight={}",
        snapshot.lcd_initialized, snapshot.lcd_backlight
    );
    let _ = writeln!(
        output,
        "BME280 raw sample: {:02X?}",
        snapshot.bme280_raw_sample
    );
    let _ = writeln!(output);

    if let Some(reading) = snapshot.reading {
        let _ = writeln!(
            output,
            "Sensor reading: temp={:.1}C hum={:.1}% pressure={}Pa",
            reading.temperature_centi_celsius as f32 / 100.0,
            reading.humidity_centi_percent as f32 / 100.0,
            reading.pressure_pascal.unwrap_or_default()
        );
    } else {
        let _ = writeln!(output, "Sensor reading: <none yet>");
    }

    let _ = writeln!(output);
    let _ = writeln!(output, "App frame");
    let _ = write!(
        output,
        "{}",
        render_frame(snapshot.rendered_frame.unwrap_or_else(TextFrame16x2::blank))
    );
    let _ = writeln!(output);
    let _ = writeln!(output, "Physical LCD");
    let _ = write!(output, "{}", render_frame(snapshot.physical_frame));
    let _ = writeln!(output);
    let _ = writeln!(output, "Recent I2C operations");
    for operation in snapshot
        .operations
        .iter()
        .rev()
        .take(8)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        let _ = writeln!(output, "  {}", format_operation(operation));
    }

    output
}

fn format_addresses(addresses: &[u8]) -> String {
    if addresses.is_empty() {
        return "<none>".to_string();
    }

    let mut output = String::new();
    for (index, address) in addresses.iter().enumerate() {
        if index != 0 {
            let _ = write!(output, ", ");
        }
        let _ = write!(output, "0x{:02X}", address);
    }
    output
}

fn render_frame(frame: TextFrame16x2) -> String {
    let mut output = String::new();
    let _ = writeln!(output, "+----------------+");
    for row in 0..2 {
        let line = core::str::from_utf8(frame.line(row)).unwrap_or("????????????????");
        let _ = writeln!(output, "|{}|", line);
    }
    let _ = writeln!(output, "+----------------+");
    output
}

fn format_operation(operation: &VirtualI2cOperation) -> String {
    match operation {
        VirtualI2cOperation::Write { addr, bytes } => {
            format!("WRITE     addr=0x{addr:02X} bytes={bytes:02X?}")
        }
        VirtualI2cOperation::Read { addr, len } => {
            format!("READ      addr=0x{addr:02X} len={len}")
        }
        VirtualI2cOperation::WriteRead { addr, bytes, len } => {
            format!("WRITE_READ addr=0x{addr:02X} bytes={bytes:02X?} len={len}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dashboard_renders_board_profile_and_devices() {
        let snapshot = DashboardSnapshot {
            board: BoardProfile::ArduinoNano,
            tick: 5,
            refresh_period_ticks: 5,
            reading: Some(EnvReading::new(2480, 4310, Some(101_325))),
            rendered_frame: Some(TextFrame16x2::from_lines("Temp    24.8C", "Hum     43.1%")),
            physical_frame: TextFrame16x2::from_lines("Temp    24.8C", "Hum     43.1%"),
            bme280_registers: Bme280ControlRegisters {
                ctrl_hum: 0x01,
                ctrl_meas: 0x27,
                config: 0x00,
            },
            bme280_raw_sample: [0; 8],
            lcd_initialized: true,
            lcd_backlight: true,
            attached_addresses: &[0x27, 0x77],
            operations: &[VirtualI2cOperation::WriteRead {
                addr: 0x77,
                bytes: vec![0xF7],
                len: 8,
            }],
        };

        let output = render_dashboard(&snapshot);

        assert!(output.contains("Arduino Nano"));
        assert!(output.contains("A4"));
        assert!(output.contains("0x27, 0x77"));
        assert!(output.contains("WRITE_READ addr=0x77"));
    }
}
