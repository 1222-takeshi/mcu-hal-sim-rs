//! Host-side BME280 mock device.

use crate::virtual_i2c::VirtualI2cDevice;
use hal_api::error::I2cError;
use std::cell::RefCell;
use std::rc::Rc;
use std::vec::Vec;

const REG_CHIP_ID: u8 = 0xD0;
const REG_CALIB_1_START: u8 = 0x88;
const REG_CALIB_2_START: u8 = 0xE1;
const REG_CTRL_HUM: u8 = 0xF2;
const REG_STATUS: u8 = 0xF3;
const REG_CTRL_MEAS: u8 = 0xF4;
const REG_CONFIG: u8 = 0xF5;
const REG_PRESS_MSB: u8 = 0xF7;
const CHIP_ID_BME280: u8 = 0x60;

const DEFAULT_CALIB_1: [u8; 26] = [
    0x70, 0x6B, 0x43, 0x67, 0x18, 0xFC, 0x7D, 0x8E, 0x43, 0xD6, 0xD0, 0x0B, 0x27, 0x0B, 0x8C, 0x00,
    0xF9, 0xFF, 0x8C, 0x3C, 0xF8, 0xC6, 0x70, 0x17, 0x00, 0x4B,
];
const DEFAULT_CALIB_2: [u8; 7] = [0x6A, 0x01, 0x00, 0x14, 0x25, 0x03, 0x1E];
const DEFAULT_RAW_SAMPLE: [u8; 8] = [0x65, 0x5A, 0xC0, 0x7E, 0xED, 0x00, 0x89, 0x98];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Bme280ControlRegisters {
    pub ctrl_hum: u8,
    pub ctrl_meas: u8,
    pub config: u8,
}

#[derive(Debug, Default)]
struct MockBme280State {
    chip_id: u8,
    status: u8,
    calib_1: [u8; 26],
    calib_2: [u8; 7],
    raw_sample: [u8; 8],
    controls: Bme280ControlRegisters,
    writes: Vec<Vec<u8>>,
}

#[derive(Clone, Debug)]
pub struct MockBme280Device {
    state: Rc<RefCell<MockBme280State>>,
}

impl MockBme280Device {
    pub fn new() -> Self {
        Self {
            state: Rc::new(RefCell::new(MockBme280State {
                chip_id: CHIP_ID_BME280,
                status: 0x00,
                calib_1: DEFAULT_CALIB_1,
                calib_2: DEFAULT_CALIB_2,
                raw_sample: DEFAULT_RAW_SAMPLE,
                controls: Bme280ControlRegisters::default(),
                writes: Vec::new(),
            })),
        }
    }

    pub fn set_chip_id(&self, chip_id: u8) {
        self.state.borrow_mut().chip_id = chip_id;
    }

    pub fn set_status(&self, status: u8) {
        self.state.borrow_mut().status = status;
    }

    pub fn set_raw_sample(&self, raw_sample: [u8; 8]) {
        self.state.borrow_mut().raw_sample = raw_sample;
    }

    pub fn raw_sample(&self) -> [u8; 8] {
        self.state.borrow().raw_sample
    }

    pub fn control_registers(&self) -> Bme280ControlRegisters {
        self.state.borrow().controls
    }

    pub fn writes(&self) -> Vec<Vec<u8>> {
        self.state.borrow().writes.clone()
    }

    fn read_register(&self, register: u8, buffer: &mut [u8]) -> Result<(), I2cError> {
        let state = self.state.borrow();
        match register {
            REG_CHIP_ID => {
                if buffer.len() != 1 {
                    return Err(I2cError::BusError);
                }
                buffer[0] = state.chip_id;
            }
            REG_STATUS => {
                if buffer.len() != 1 {
                    return Err(I2cError::BusError);
                }
                buffer[0] = state.status;
            }
            REG_CALIB_1_START => {
                if buffer.len() != state.calib_1.len() {
                    return Err(I2cError::BusError);
                }
                buffer.copy_from_slice(&state.calib_1);
            }
            REG_CALIB_2_START => {
                if buffer.len() != state.calib_2.len() {
                    return Err(I2cError::BusError);
                }
                buffer.copy_from_slice(&state.calib_2);
            }
            REG_PRESS_MSB => {
                if buffer.len() != state.raw_sample.len() {
                    return Err(I2cError::BusError);
                }
                buffer.copy_from_slice(&state.raw_sample);
            }
            _ => return Err(I2cError::InvalidAddress),
        }
        Ok(())
    }
}

pub fn demo_raw_samples() -> Vec<[u8; 8]> {
    vec![
        DEFAULT_RAW_SAMPLE,
        [0x66, 0x1A, 0x80, 0x7F, 0x20, 0x10, 0x8A, 0x10],
        [0x64, 0xE0, 0x40, 0x7E, 0x90, 0x80, 0x88, 0xC0],
        [0x67, 0x00, 0xF0, 0x7F, 0x60, 0xC0, 0x8B, 0x30],
    ]
}

impl Default for MockBme280Device {
    fn default() -> Self {
        Self::new()
    }
}

impl VirtualI2cDevice for MockBme280Device {
    fn write(&mut self, bytes: &[u8]) -> Result<(), I2cError> {
        let mut state = self.state.borrow_mut();
        state.writes.push(bytes.to_vec());

        if bytes.len() == 2 {
            match bytes[0] {
                REG_CTRL_HUM => state.controls.ctrl_hum = bytes[1],
                REG_CTRL_MEAS => state.controls.ctrl_meas = bytes[1],
                REG_CONFIG => state.controls.config = bytes[1],
                _ => {}
            }
        }

        Ok(())
    }

    fn write_read(&mut self, bytes: &[u8], buffer: &mut [u8]) -> Result<(), I2cError> {
        let register = *bytes.first().ok_or(I2cError::BusError)?;
        self.read_register(register, buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::virtual_i2c::VirtualI2cDevice;

    #[test]
    fn mock_bme280_exposes_default_chip_id_and_sample_registers() {
        let mut device = MockBme280Device::new();
        let mut chip_id = [0u8; 1];
        let mut sample = [0u8; 8];

        device.write_read(&[REG_CHIP_ID], &mut chip_id).unwrap();
        device.write_read(&[REG_PRESS_MSB], &mut sample).unwrap();

        assert_eq!(chip_id, [CHIP_ID_BME280]);
        assert_eq!(sample, DEFAULT_RAW_SAMPLE);
    }

    #[test]
    fn mock_bme280_tracks_control_register_writes() {
        let mut device = MockBme280Device::new();

        device.write(&[REG_CTRL_HUM, 0x01]).unwrap();
        device.write(&[REG_CTRL_MEAS, 0x27]).unwrap();
        device.write(&[REG_CONFIG, 0x10]).unwrap();

        assert_eq!(
            device.control_registers(),
            Bme280ControlRegisters {
                ctrl_hum: 0x01,
                ctrl_meas: 0x27,
                config: 0x10,
            }
        );
    }

    #[test]
    fn mock_bme280_allows_overriding_status_and_raw_sample() {
        let device = MockBme280Device::new();
        let mut status = [0u8; 1];
        let mut sample = [0u8; 8];

        device.set_status(0x08);
        device.set_raw_sample([1, 2, 3, 4, 5, 6, 7, 8]);

        let mut handle = device;
        handle.write_read(&[REG_STATUS], &mut status).unwrap();
        handle.write_read(&[REG_PRESS_MSB], &mut sample).unwrap();

        assert_eq!(status, [0x08]);
        assert_eq!(sample, [1, 2, 3, 4, 5, 6, 7, 8]);
    }
}
