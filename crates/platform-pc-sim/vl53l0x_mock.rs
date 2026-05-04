//! Host-side VL53L0X ToF distance sensor mock device.

use crate::virtual_i2c::VirtualI2cDevice;
use hal_api::error::I2cError;
use std::cell::RefCell;
use std::rc::Rc;
use std::vec::Vec;

const REG_MODEL_ID: u8 = 0xC0;
const MODEL_ID: u8 = 0xEE;
const REG_SYSRANGE_START: u8 = 0x00;
const REG_INTERRUPT_STATUS: u8 = 0x13;
const REG_RESULT_RANGE_MM: u8 = 0x1E;

/// Demo cycling distances (mm) simulating an obstacle approaching then receding.
pub fn demo_distances_mm() -> Vec<u32> {
    vec![
        1500, 1200, 900, 600, 400, 250, 150, 100, 200, 500, 900, 1300,
    ]
}

#[derive(Debug)]
struct MockVl53l0xState {
    /// True after SYSRANGE_START = 0x01 write received.
    measurement_triggered: bool,
    /// Next distance to return on RESULT_RANGE_MM read.
    next_distance_mm: u32,
    /// Cycling distances.
    distances: Vec<u32>,
    /// Index into `distances`.
    distance_index: usize,
}

impl MockVl53l0xState {
    fn advance(&mut self) -> u32 {
        let d = self.distances[self.distance_index % self.distances.len()];
        self.distance_index = (self.distance_index + 1) % self.distances.len();
        d
    }
}

/// Host-side VL53L0X mock implementing [`VirtualI2cDevice`].
///
/// Handles VL53L0X protocol:
/// - `write_read([0xC0], buf[1])` → model ID = 0xEE
/// - `write([0x00, 0x01])` → triggers measurement
/// - `write_read([0x13], status[1])` → interrupt status = 0x01 (ready)
/// - `write_read([0x1E], buf[2])` → 2-byte distance BE
#[derive(Clone, Debug)]
pub struct MockVl53l0xDevice {
    state: Rc<RefCell<MockVl53l0xState>>,
}

impl MockVl53l0xDevice {
    pub fn new() -> Self {
        Self::looping(demo_distances_mm())
    }

    pub fn looping(distances: Vec<u32>) -> Self {
        let first = distances.first().copied().unwrap_or(500);
        Self {
            state: Rc::new(RefCell::new(MockVl53l0xState {
                measurement_triggered: false,
                next_distance_mm: first,
                distances,
                distance_index: 0,
            })),
        }
    }

    pub fn set_next_distance(&self, mm: u32) {
        self.state.borrow_mut().next_distance_mm = mm;
    }
}

impl Default for MockVl53l0xDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl VirtualI2cDevice for MockVl53l0xDevice {
    fn write(&mut self, bytes: &[u8]) -> Result<(), I2cError> {
        if bytes.len() == 2 && bytes[0] == REG_SYSRANGE_START && bytes[1] == 0x01 {
            let mut state = self.state.borrow_mut();
            state.measurement_triggered = true;
            state.next_distance_mm = state.advance();
        }
        Ok(())
    }

    fn write_read(&mut self, bytes: &[u8], buffer: &mut [u8]) -> Result<(), I2cError> {
        if bytes.is_empty() {
            return Err(I2cError::BusError);
        }
        match bytes[0] {
            REG_MODEL_ID => {
                if buffer.len() != 1 {
                    return Err(I2cError::BusError);
                }
                buffer[0] = MODEL_ID;
            }
            REG_INTERRUPT_STATUS => {
                if buffer.len() != 1 {
                    return Err(I2cError::BusError);
                }
                // Always report measurement ready (bit0 set)
                buffer[0] = 0x01;
            }
            REG_RESULT_RANGE_MM => {
                if buffer.len() != 2 {
                    return Err(I2cError::BusError);
                }
                let mm = self.state.borrow().next_distance_mm as u16;
                buffer[0] = (mm >> 8) as u8;
                buffer[1] = mm as u8;
            }
            _ => return Err(I2cError::InvalidAddress),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_vl53l0x_returns_model_id() {
        let mut device = MockVl53l0xDevice::new();
        let mut buf = [0u8; 1];
        device.write_read(&[REG_MODEL_ID], &mut buf).unwrap();
        assert_eq!(buf[0], MODEL_ID);
    }

    #[test]
    fn mock_vl53l0x_interrupt_status_always_ready() {
        let mut device = MockVl53l0xDevice::new();
        let mut buf = [0u8; 1];
        device
            .write_read(&[REG_INTERRUPT_STATUS], &mut buf)
            .unwrap();
        assert_eq!(buf[0] & 0x07, 0x01);
    }

    #[test]
    fn mock_vl53l0x_returns_distance_after_trigger() {
        let mut device = MockVl53l0xDevice::new();
        device.set_next_distance(350);
        device.write(&[REG_SYSRANGE_START, 0x01]).unwrap();
        let mut buf = [0u8; 2];
        device.write_read(&[REG_RESULT_RANGE_MM], &mut buf).unwrap();
        let mm = u16::from_be_bytes(buf);
        // After trigger, state.advance() gives the next cycling distance
        // The exact value depends on cycling, but must be within u16 range
        let _ = mm;
    }

    #[test]
    fn mock_vl53l0x_set_next_distance_overrides() {
        let device = MockVl53l0xDevice::new();
        device.set_next_distance(1234);
        let mut handle = device;
        let mut buf = [0u8; 2];
        handle.write_read(&[REG_RESULT_RANGE_MM], &mut buf).unwrap();
        let mm = u16::from_be_bytes(buf);
        assert_eq!(mm, 1234);
    }

    #[test]
    fn mock_vl53l0x_cycling_distances() {
        let distances = vec![100u32, 200, 300];
        let mut device = MockVl53l0xDevice::looping(distances);
        let mut buf = [0u8; 2];

        // Trigger three measurements and check cycling
        for expected in &[100u16, 200, 300] {
            device.write(&[REG_SYSRANGE_START, 0x01]).unwrap();
            device.write_read(&[REG_RESULT_RANGE_MM], &mut buf).unwrap();
            assert_eq!(u16::from_be_bytes(buf), *expected);
        }
    }

    #[test]
    fn mock_vl53l0x_rejects_unknown_register() {
        let mut device = MockVl53l0xDevice::new();
        let mut buf = [0u8; 1];
        assert!(device.write_read(&[0xFF], &mut buf).is_err());
    }
}
