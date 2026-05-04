//! Host-side SGP30 gas sensor mock device.

use crate::virtual_i2c::VirtualI2cDevice;
use hal_api::error::I2cError;
use std::cell::RefCell;
use std::rc::Rc;
use std::vec::Vec;

const CMD_INIT: [u8; 2] = [0x20, 0x03];
const CMD_MEASURE: [u8; 2] = [0x20, 0x08];

/// A single CO₂/VOC reading from the SGP30 mock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MockGasReading {
    /// CO₂ equivalent in ppm.
    pub co2_ppm: u16,
    /// TVOC in ppb.
    pub voc_ppb: u16,
}

impl MockGasReading {
    pub fn new(co2_ppm: u16, voc_ppb: u16) -> Self {
        Self { co2_ppm, voc_ppb }
    }
}

/// Demo cycling gas readings simulating indoor air quality changes.
pub fn demo_gas_readings() -> Vec<MockGasReading> {
    vec![
        MockGasReading::new(400, 0),
        MockGasReading::new(450, 10),
        MockGasReading::new(520, 25),
        MockGasReading::new(680, 75),
        MockGasReading::new(900, 150),
        MockGasReading::new(1100, 220),
        MockGasReading::new(850, 120),
        MockGasReading::new(600, 50),
    ]
}

#[derive(Debug)]
struct MockSgp30State {
    /// True after CMD_INIT has been received.
    initialized: bool,
    /// Next measurement data (set when CMD_MEASURE is received).
    pending: Option<[u8; 6]>,
    /// Current cycling reading index.
    reading_index: usize,
    /// Cycling readings to loop through.
    readings: Vec<MockGasReading>,
    /// Number of write operations received.
    write_count: usize,
}

impl MockSgp30State {
    fn next_measurement_bytes(&mut self) -> [u8; 6] {
        let r = self.readings[self.reading_index % self.readings.len()];
        self.reading_index = (self.reading_index + 1) % self.readings.len();
        [
            (r.co2_ppm >> 8) as u8,
            r.co2_ppm as u8,
            0x00, // CRC placeholder
            (r.voc_ppb >> 8) as u8,
            r.voc_ppb as u8,
            0x00, // CRC placeholder
        ]
    }
}

/// Host-side SGP30 mock implementing [`VirtualI2cDevice`].
///
/// Handles SGP30's command-then-read protocol:
/// - `write([0x20, 0x03])` → init_air_quality
/// - `write([0x20, 0x08])` → triggers next measurement; result available via `read()`
/// - `read(buf[6])` → returns `[CO2_H, CO2_L, CRC, VOC_H, VOC_L, CRC]`
#[derive(Clone, Debug)]
pub struct MockSgp30Device {
    state: Rc<RefCell<MockSgp30State>>,
}

impl MockSgp30Device {
    pub fn new() -> Self {
        Self::looping(demo_gas_readings())
    }

    pub fn looping(readings: Vec<MockGasReading>) -> Self {
        Self {
            state: Rc::new(RefCell::new(MockSgp30State {
                initialized: false,
                pending: None,
                reading_index: 0,
                readings,
                write_count: 0,
            })),
        }
    }

    pub fn is_initialized(&self) -> bool {
        self.state.borrow().initialized
    }

    pub fn write_count(&self) -> usize {
        self.state.borrow().write_count
    }
}

impl Default for MockSgp30Device {
    fn default() -> Self {
        Self::new()
    }
}

impl VirtualI2cDevice for MockSgp30Device {
    fn write(&mut self, bytes: &[u8]) -> Result<(), I2cError> {
        let mut state = self.state.borrow_mut();
        state.write_count += 1;
        if bytes == CMD_INIT {
            state.initialized = true;
        } else if bytes == CMD_MEASURE {
            let measurement = state.next_measurement_bytes();
            state.pending = Some(measurement);
        }
        Ok(())
    }

    fn read(&mut self, buffer: &mut [u8]) -> Result<(), I2cError> {
        let mut state = self.state.borrow_mut();
        if buffer.len() != 6 {
            return Err(I2cError::BusError);
        }
        let data = state.pending.take().ok_or(I2cError::BusError)?;
        buffer.copy_from_slice(&data);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_sgp30_init_marks_initialized() {
        let mut device = MockSgp30Device::new();
        assert!(!device.is_initialized());
        device.write(&CMD_INIT).unwrap();
        assert!(device.is_initialized());
    }

    #[test]
    fn mock_sgp30_returns_cycling_readings() {
        let readings = vec![MockGasReading::new(400, 0), MockGasReading::new(800, 100)];
        let mut device = MockSgp30Device::looping(readings);
        device.write(&CMD_INIT).unwrap();

        device.write(&CMD_MEASURE).unwrap();
        let mut buf = [0u8; 6];
        device.read(&mut buf).unwrap();
        let co2 = u16::from_be_bytes([buf[0], buf[1]]);
        let voc = u16::from_be_bytes([buf[3], buf[4]]);
        assert_eq!(co2, 400);
        assert_eq!(voc, 0);

        device.write(&CMD_MEASURE).unwrap();
        device.read(&mut buf).unwrap();
        let co2 = u16::from_be_bytes([buf[0], buf[1]]);
        assert_eq!(co2, 800);
    }

    #[test]
    fn mock_sgp30_read_without_measure_fails() {
        let mut device = MockSgp30Device::new();
        let mut buf = [0u8; 6];
        // No CMD_MEASURE → pending is None → error
        assert!(device.read(&mut buf).is_err());
    }

    #[test]
    fn mock_sgp30_read_wrong_buffer_size_fails() {
        let mut device = MockSgp30Device::new();
        device.write(&CMD_MEASURE).unwrap();
        let mut buf = [0u8; 4]; // wrong size
        assert!(device.read(&mut buf).is_err());
    }

    #[test]
    fn mock_sgp30_tracks_write_count() {
        let mut device = MockSgp30Device::new();
        device.write(&CMD_INIT).unwrap();
        device.write(&CMD_MEASURE).unwrap();
        device.write(&CMD_MEASURE).unwrap();
        assert_eq!(device.write_count(), 3);
    }
}
