//! Host-side DS3231 RTC mock device.

use crate::virtual_i2c::VirtualI2cDevice;
use hal_api::error::I2cError;
use std::cell::RefCell;
use std::rc::Rc;
use std::vec::Vec;

const REG_SECONDS: u8 = 0x00;

fn dec_to_bcd(dec: u8) -> u8 {
    ((dec / 10) << 4) | (dec % 10)
}

/// A single RTC timestamp in BCD 24h format, matching DS3231 registers 0x00–0x06.
///
/// Layout: `[sec, min, hour, dow, day, month, year_offset]`
/// All fields are BCD and use the 24h convention (no AM/PM bit).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MockRtcTimestamp {
    /// BCD seconds (0–59)
    pub sec: u8,
    /// BCD minutes (0–59)
    pub min: u8,
    /// BCD hours, 24h mode (0–23)
    pub hour: u8,
    /// Day of week (1–7, not decoded by driver)
    pub dow: u8,
    /// BCD date (1–31)
    pub day: u8,
    /// BCD month (1–12)
    pub month: u8,
    /// BCD year offset from 2000 (0–99)
    pub year_offset: u8,
}

impl MockRtcTimestamp {
    /// Construct from decimal values (auto-converted to BCD).
    pub fn from_decimal(
        sec: u8,
        min: u8,
        hour: u8,
        dow: u8,
        day: u8,
        month: u8,
        year_offset: u8,
    ) -> Self {
        Self {
            sec: dec_to_bcd(sec),
            min: dec_to_bcd(min),
            hour: dec_to_bcd(hour),
            dow,
            day: dec_to_bcd(day),
            month: dec_to_bcd(month),
            year_offset: dec_to_bcd(year_offset),
        }
    }

    fn to_register_bytes(self) -> [u8; 7] {
        [
            self.sec,
            self.min,
            self.hour,
            self.dow,
            self.day,
            self.month,
            self.year_offset,
        ]
    }
}

impl Default for MockRtcTimestamp {
    fn default() -> Self {
        // 2025-05-04 12:00:00 (BCD)
        Self::from_decimal(0, 0, 12, 1, 4, 5, 25)
    }
}

/// Demo cycling timestamps for animation in the dashboard.
pub fn demo_timestamps() -> Vec<MockRtcTimestamp> {
    vec![
        MockRtcTimestamp::from_decimal(0, 0, 9, 1, 4, 5, 25),
        MockRtcTimestamp::from_decimal(15, 0, 9, 1, 4, 5, 25),
        MockRtcTimestamp::from_decimal(30, 0, 9, 1, 4, 5, 25),
        MockRtcTimestamp::from_decimal(45, 0, 9, 1, 4, 5, 25),
        MockRtcTimestamp::from_decimal(0, 1, 9, 1, 4, 5, 25),
        MockRtcTimestamp::from_decimal(15, 1, 9, 1, 4, 5, 25),
        MockRtcTimestamp::from_decimal(30, 1, 9, 1, 4, 5, 25),
        MockRtcTimestamp::from_decimal(45, 1, 9, 1, 4, 5, 25),
    ]
}

#[derive(Debug)]
struct MockDs3231State {
    current: MockRtcTimestamp,
    last_set: Option<[u8; 7]>,
}

/// Host-side DS3231 mock implementing [`VirtualI2cDevice`].
///
/// Responds to DS3231's register map:
/// - `write_read(&[0x00], buf[7])` → returns 7-byte BCD register frame.
/// - `write(buf[8..])` starting at register 0x00 → stores `set_datetime` payload.
#[derive(Clone, Debug)]
pub struct MockDs3231Device {
    state: Rc<RefCell<MockDs3231State>>,
}

impl MockDs3231Device {
    pub fn new() -> Self {
        Self {
            state: Rc::new(RefCell::new(MockDs3231State {
                current: MockRtcTimestamp::default(),
                last_set: None,
            })),
        }
    }

    pub fn set_timestamp(&self, ts: MockRtcTimestamp) {
        self.state.borrow_mut().current = ts;
    }

    pub fn last_set_payload(&self) -> Option<[u8; 7]> {
        self.state.borrow().last_set
    }
}

impl Default for MockDs3231Device {
    fn default() -> Self {
        Self::new()
    }
}

impl VirtualI2cDevice for MockDs3231Device {
    fn write(&mut self, bytes: &[u8]) -> Result<(), I2cError> {
        // set_datetime writes: [0x00, sec, min, hour, dow, day, month, year]
        if bytes.len() == 8 && bytes[0] == REG_SECONDS {
            let mut payload = [0u8; 7];
            payload.copy_from_slice(&bytes[1..8]);
            self.state.borrow_mut().last_set = Some(payload);
        }
        Ok(())
    }

    fn write_read(&mut self, bytes: &[u8], buffer: &mut [u8]) -> Result<(), I2cError> {
        if bytes.is_empty() {
            return Err(I2cError::BusError);
        }
        if bytes[0] == REG_SECONDS && buffer.len() == 7 {
            let regs = self.state.borrow().current.to_register_bytes();
            buffer.copy_from_slice(&regs);
            return Ok(());
        }
        Err(I2cError::InvalidAddress)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_ds3231_returns_default_datetime() {
        let mut device = MockDs3231Device::new();
        let mut buf = [0u8; 7];
        device.write_read(&[0x00], &mut buf).unwrap();
        // Default: 2025-05-04 12:00:00
        // hour BCD = 0x12
        assert_eq!(buf[2], 0x12);
        // month BCD = 0x05
        assert_eq!(buf[5], 0x05);
        // year BCD = 0x25
        assert_eq!(buf[6], 0x25);
    }

    #[test]
    fn mock_ds3231_set_timestamp_affects_read() {
        let device = MockDs3231Device::new();
        device.set_timestamp(MockRtcTimestamp::from_decimal(30, 45, 15, 1, 10, 6, 25));

        let mut handle = device;
        let mut buf = [0u8; 7];
        handle.write_read(&[0x00], &mut buf).unwrap();

        assert_eq!(buf[0], 0x30); // sec BCD
        assert_eq!(buf[1], 0x45); // min BCD
        assert_eq!(buf[2], 0x15); // hour BCD
        assert_eq!(buf[4], 0x10); // day BCD
    }

    #[test]
    fn mock_ds3231_records_set_datetime_write() {
        let mut device = MockDs3231Device::new();
        // set_datetime payload: reg=0x00, sec=0x45, min=0x30, hour=0x12, dow=0x01, day=0x04, month=0x05, year=0x25
        device
            .write(&[0x00, 0x45, 0x30, 0x12, 0x01, 0x04, 0x05, 0x25])
            .unwrap();
        let last = device.last_set_payload().unwrap();
        assert_eq!(last[0], 0x45); // sec
        assert_eq!(last[1], 0x30); // min
        assert_eq!(last[2], 0x12); // hour
    }

    #[test]
    fn mock_ds3231_rejects_unknown_register() {
        let mut device = MockDs3231Device::new();
        let mut buf = [0u8; 1];
        assert!(device.write_read(&[0xFF], &mut buf).is_err());
    }
}
