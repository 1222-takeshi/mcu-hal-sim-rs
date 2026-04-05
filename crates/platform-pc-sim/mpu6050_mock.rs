//! Host-side MPU6050 mock device.

use crate::virtual_i2c::VirtualI2cDevice;
use hal_api::error::I2cError;
use std::cell::RefCell;
use std::rc::Rc;
use std::vec::Vec;

const REG_ACCEL_XOUT_H: u8 = 0x3B;
const REG_CONFIG: u8 = 0x1A;
const REG_GYRO_CONFIG: u8 = 0x1B;
const REG_ACCEL_CONFIG: u8 = 0x1C;
const REG_PWR_MGMT_1: u8 = 0x6B;
const REG_WHO_AM_I: u8 = 0x75;
const WHO_AM_I_MPU6050: u8 = 0x68;

const DEFAULT_RAW_FRAME: [u8; 14] = [
    0x40, 0x00, 0xE0, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x83, 0xFF, 0x7D, 0x00, 0x00,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Mpu6050ControlRegisters {
    pub power_management_1: u8,
    pub config: u8,
    pub gyro_config: u8,
    pub accel_config: u8,
}

#[derive(Debug, Default)]
struct MockMpu6050State {
    identity: u8,
    raw_frame: [u8; 14],
    controls: Mpu6050ControlRegisters,
    writes: Vec<Vec<u8>>,
}

#[derive(Clone, Debug)]
pub struct MockMpu6050Device {
    state: Rc<RefCell<MockMpu6050State>>,
}

impl MockMpu6050Device {
    pub fn new() -> Self {
        Self {
            state: Rc::new(RefCell::new(MockMpu6050State {
                identity: WHO_AM_I_MPU6050,
                raw_frame: DEFAULT_RAW_FRAME,
                controls: Mpu6050ControlRegisters::default(),
                writes: Vec::new(),
            })),
        }
    }

    pub fn set_identity(&self, identity: u8) {
        self.state.borrow_mut().identity = identity;
    }

    pub fn set_raw_frame(&self, raw_frame: [u8; 14]) {
        self.state.borrow_mut().raw_frame = raw_frame;
    }

    pub fn raw_frame(&self) -> [u8; 14] {
        self.state.borrow().raw_frame
    }

    pub fn control_registers(&self) -> Mpu6050ControlRegisters {
        self.state.borrow().controls
    }

    pub fn writes(&self) -> Vec<Vec<u8>> {
        self.state.borrow().writes.clone()
    }

    fn read_register(&self, register: u8, buffer: &mut [u8]) -> Result<(), I2cError> {
        let state = self.state.borrow();
        match register {
            REG_WHO_AM_I => {
                if buffer.len() != 1 {
                    return Err(I2cError::BusError);
                }
                buffer[0] = state.identity;
            }
            REG_ACCEL_XOUT_H => {
                if buffer.len() != state.raw_frame.len() {
                    return Err(I2cError::BusError);
                }
                buffer.copy_from_slice(&state.raw_frame);
            }
            _ => return Err(I2cError::InvalidAddress),
        }
        Ok(())
    }
}

pub fn demo_raw_frames() -> Vec<[u8; 14]> {
    vec![
        DEFAULT_RAW_FRAME,
        [
            0x3A, 0x98, 0xF6, 0x00, 0x41, 0x00, 0x00, 0x44, 0x00, 0x00, 0x01, 0x06, 0xFF, 0x7D,
        ],
        [
            0x44, 0x00, 0x08, 0x00, 0x3F, 0x60, 0x00, 0x88, 0xFF, 0x06, 0x00, 0x00, 0x00, 0x83,
        ],
        [
            0x3F, 0x00, 0x08, 0xF0, 0x40, 0x20, 0x00, 0x22, 0x00, 0x50, 0xFF, 0xB0, 0x00, 0x00,
        ],
    ]
}

impl Default for MockMpu6050Device {
    fn default() -> Self {
        Self::new()
    }
}

impl VirtualI2cDevice for MockMpu6050Device {
    fn write(&mut self, bytes: &[u8]) -> Result<(), I2cError> {
        let mut state = self.state.borrow_mut();
        state.writes.push(bytes.to_vec());

        if bytes.len() == 2 {
            match bytes[0] {
                REG_PWR_MGMT_1 => state.controls.power_management_1 = bytes[1],
                REG_CONFIG => state.controls.config = bytes[1],
                REG_GYRO_CONFIG => state.controls.gyro_config = bytes[1],
                REG_ACCEL_CONFIG => state.controls.accel_config = bytes[1],
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
    fn mock_mpu6050_exposes_identity_and_raw_frame() {
        let mut device = MockMpu6050Device::new();
        let mut identity = [0u8; 1];
        let mut raw_frame = [0u8; 14];

        device.write_read(&[REG_WHO_AM_I], &mut identity).unwrap();
        device
            .write_read(&[REG_ACCEL_XOUT_H], &mut raw_frame)
            .unwrap();

        assert_eq!(identity, [WHO_AM_I_MPU6050]);
        assert_eq!(raw_frame, DEFAULT_RAW_FRAME);
    }

    #[test]
    fn mock_mpu6050_tracks_control_register_writes() {
        let mut device = MockMpu6050Device::new();

        device.write(&[REG_PWR_MGMT_1, 0x00]).unwrap();
        device.write(&[REG_CONFIG, 0x03]).unwrap();
        device.write(&[REG_GYRO_CONFIG, 0x08]).unwrap();
        device.write(&[REG_ACCEL_CONFIG, 0x10]).unwrap();

        assert_eq!(
            device.control_registers(),
            Mpu6050ControlRegisters {
                power_management_1: 0x00,
                config: 0x03,
                gyro_config: 0x08,
                accel_config: 0x10,
            }
        );
    }

    #[test]
    fn mock_mpu6050_allows_overrides() {
        let device = MockMpu6050Device::new();
        let mut identity = [0u8; 1];
        let mut raw_frame = [0u8; 14];

        device.set_identity(0x70);
        device.set_raw_frame([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14]);

        let mut handle = device;
        handle.write_read(&[REG_WHO_AM_I], &mut identity).unwrap();
        handle
            .write_read(&[REG_ACCEL_XOUT_H], &mut raw_frame)
            .unwrap();

        assert_eq!(identity, [0x70]);
        assert_eq!(raw_frame, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14]);
    }
}
