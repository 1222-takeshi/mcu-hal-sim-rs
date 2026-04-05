//! MPU6050 IMU driver.

use hal_api::error::{I2cError, SensorError};
use hal_api::i2c::I2cBus;
use hal_api::imu::{ImuReading, ImuSensor};

const REG_ACCEL_XOUT_H: u8 = 0x3B;
const REG_PWR_MGMT_1: u8 = 0x6B;
const REG_CONFIG: u8 = 0x1A;
const REG_GYRO_CONFIG: u8 = 0x1B;
const REG_ACCEL_CONFIG: u8 = 0x1C;
const REG_WHO_AM_I: u8 = 0x75;
const WHO_AM_I_MPU6050: u8 = 0x68;

pub const MPU6050_ADDRESS_PRIMARY: u8 = 0x68;
pub const MPU6050_ADDRESS_SECONDARY: u8 = 0x69;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mpu6050Config {
    pub address: u8,
    pub power_management_1: u8,
    pub config: u8,
    pub gyro_config: u8,
    pub accel_config: u8,
}

impl Default for Mpu6050Config {
    fn default() -> Self {
        Self {
            address: MPU6050_ADDRESS_PRIMARY,
            power_management_1: 0x00,
            config: 0x03,
            gyro_config: 0x00,
            accel_config: 0x00,
        }
    }
}

/// MPU6050 driver.
pub struct Mpu6050Sensor<B> {
    bus: B,
    config: Mpu6050Config,
    initialized: bool,
}

impl<B> Mpu6050Sensor<B> {
    pub fn new(bus: B) -> Self {
        Self::new_with_config(bus, Mpu6050Config::default())
    }

    pub fn new_with_address(bus: B, address: u8) -> Self {
        Self::new_with_config(
            bus,
            Mpu6050Config {
                address,
                ..Mpu6050Config::default()
            },
        )
    }

    pub fn new_with_config(bus: B, config: Mpu6050Config) -> Self {
        Self {
            bus,
            config,
            initialized: false,
        }
    }

    pub fn address(&self) -> u8 {
        self.config.address
    }

    pub fn config(&self) -> Mpu6050Config {
        self.config
    }

    #[cfg(test)]
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}

impl<B> Mpu6050Sensor<B>
where
    B: I2cBus<Error = I2cError>,
{
    fn initialize(&mut self) -> Result<(), SensorError> {
        if self.initialized {
            return Ok(());
        }

        if self.read_u8(REG_WHO_AM_I)? != WHO_AM_I_MPU6050 {
            return Err(SensorError::InvalidReading);
        }

        self.write_register(REG_PWR_MGMT_1, self.config.power_management_1)?;
        self.write_register(REG_CONFIG, self.config.config)?;
        self.write_register(REG_GYRO_CONFIG, self.config.gyro_config)?;
        self.write_register(REG_ACCEL_CONFIG, self.config.accel_config)?;
        self.initialized = true;
        Ok(())
    }

    fn read_u8(&mut self, register: u8) -> Result<u8, SensorError> {
        let mut value = [0u8; 1];
        self.read_registers(register, &mut value)?;
        Ok(value[0])
    }

    fn read_registers(&mut self, register: u8, buffer: &mut [u8]) -> Result<(), SensorError> {
        self.bus
            .write_read(self.config.address, &[register], buffer)
            .map_err(map_sensor_error)
    }

    fn write_register(&mut self, register: u8, value: u8) -> Result<(), SensorError> {
        self.bus
            .write(self.config.address, &[register, value])
            .map_err(map_sensor_error)
    }
}

impl<B> ImuSensor for Mpu6050Sensor<B>
where
    B: I2cBus<Error = I2cError>,
{
    type Error = SensorError;

    fn read_imu(&mut self) -> Result<ImuReading, Self::Error> {
        self.initialize()?;

        let mut raw = [0u8; 14];
        self.read_registers(REG_ACCEL_XOUT_H, &mut raw)?;

        let accel_x = i16::from_be_bytes([raw[0], raw[1]]);
        let accel_y = i16::from_be_bytes([raw[2], raw[3]]);
        let accel_z = i16::from_be_bytes([raw[4], raw[5]]);
        let temperature = i16::from_be_bytes([raw[6], raw[7]]);
        let gyro_x = i16::from_be_bytes([raw[8], raw[9]]);
        let gyro_y = i16::from_be_bytes([raw[10], raw[11]]);
        let gyro_z = i16::from_be_bytes([raw[12], raw[13]]);

        Ok(ImuReading::new(
            [
                accel_to_mg(accel_x, self.config.accel_config),
                accel_to_mg(accel_y, self.config.accel_config),
                accel_to_mg(accel_z, self.config.accel_config),
            ],
            [
                gyro_to_mdps(gyro_x, self.config.gyro_config),
                gyro_to_mdps(gyro_y, self.config.gyro_config),
                gyro_to_mdps(gyro_z, self.config.gyro_config),
            ],
            Some(temperature_to_centi_celsius(temperature)),
        ))
    }
}

fn accel_sensitivity(accel_config: u8) -> i32 {
    match accel_config & 0x18 {
        0x00 => 16_384,
        0x08 => 8_192,
        0x10 => 4_096,
        _ => 2_048,
    }
}

fn gyro_sensitivity(gyro_config: u8) -> i32 {
    match gyro_config & 0x18 {
        0x00 => 131,
        0x08 => 65,
        0x10 => 33,
        _ => 16,
    }
}

fn accel_to_mg(raw: i16, accel_config: u8) -> i16 {
    ((i32::from(raw) * 1000) / accel_sensitivity(accel_config)) as i16
}

fn gyro_to_mdps(raw: i16, gyro_config: u8) -> i32 {
    (i32::from(raw) * 1000) / gyro_sensitivity(gyro_config)
}

fn temperature_to_centi_celsius(raw: i16) -> i16 {
    let scaled = (i32::from(raw) * 100 + 170) / 340;
    (scaled + 3_653) as i16
}

fn map_sensor_error(error: I2cError) -> SensorError {
    match error {
        I2cError::InvalidAddress => SensorError::NotInitialized,
        I2cError::BusError | I2cError::Timeout => SensorError::BusError,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate std;
    use hal_api::error::I2cError;
    use self::std::cell::RefCell;
    use self::std::collections::BTreeMap;
    use self::std::rc::Rc;
    use self::std::vec;
    use self::std::vec::Vec;

    #[derive(Clone, Default)]
    struct RecordingI2c {
        state: Rc<RefCell<RecordingI2cState>>,
    }

    #[derive(Default)]
    struct RecordingI2cState {
        writes: Vec<(u8, Vec<u8>)>,
        registers: BTreeMap<u8, Vec<u8>>,
    }

    impl RecordingI2c {
        fn with_register(register: u8, bytes: &[u8]) -> Self {
            let bus = Self::default();
            bus.state
                .borrow_mut()
                .registers
                .insert(register, bytes.to_vec());
            bus
        }

        fn set_register(&self, register: u8, bytes: &[u8]) {
            self.state
                .borrow_mut()
                .registers
                .insert(register, bytes.to_vec());
        }

        fn writes(&self) -> Vec<(u8, Vec<u8>)> {
            self.state.borrow().writes.clone()
        }
    }

    impl I2cBus for RecordingI2c {
        type Error = I2cError;

        fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
            self.state.borrow_mut().writes.push((addr, bytes.to_vec()));
            Ok(())
        }

        fn read(&mut self, _addr: u8, _buffer: &mut [u8]) -> Result<(), Self::Error> {
            Err(I2cError::BusError)
        }

        fn write_read(
            &mut self,
            _addr: u8,
            bytes: &[u8],
            buffer: &mut [u8],
        ) -> Result<(), Self::Error> {
            let register = *bytes.first().ok_or(I2cError::BusError)?;
            let state = self.state.borrow();
            let value = state.registers.get(&register).ok_or(I2cError::InvalidAddress)?;
            if value.len() != buffer.len() {
                return Err(I2cError::BusError);
            }
            buffer.copy_from_slice(value);
            Ok(())
        }
    }

    fn sample_frame() -> [u8; 14] {
        [
            0x40, 0x00, 0xE0, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x83, 0xFF, 0x7D, 0x00, 0x00,
        ]
    }

    #[test]
    fn mpu6050_sensor_initializes_and_reads_imu() {
        let bus = RecordingI2c::with_register(REG_WHO_AM_I, &[WHO_AM_I_MPU6050]);
        bus.set_register(REG_ACCEL_XOUT_H, &sample_frame());
        let mut sensor = Mpu6050Sensor::new(bus.clone());

        let reading = sensor.read_imu().unwrap();

        assert_eq!(reading.accel_mg, [1000, -500, 1000]);
        assert_eq!(reading.gyro_mdps, [1000, -1000, 0]);
        assert_eq!(reading.temperature_centi_celsius, Some(3653));
        assert!(sensor.is_initialized());
        assert_eq!(
            bus.writes(),
            vec![
                (MPU6050_ADDRESS_PRIMARY, vec![REG_PWR_MGMT_1, 0x00]),
                (MPU6050_ADDRESS_PRIMARY, vec![REG_CONFIG, 0x03]),
                (MPU6050_ADDRESS_PRIMARY, vec![REG_GYRO_CONFIG, 0x00]),
                (MPU6050_ADDRESS_PRIMARY, vec![REG_ACCEL_CONFIG, 0x00]),
            ]
        );
    }

    #[test]
    fn mpu6050_sensor_rejects_unexpected_identity() {
        let bus = RecordingI2c::with_register(REG_WHO_AM_I, &[0x42]);
        let mut sensor = Mpu6050Sensor::new(bus);

        assert_eq!(sensor.read_imu(), Err(SensorError::InvalidReading));
    }

    #[test]
    fn mpu6050_sensor_uses_custom_config_registers() {
        let config = Mpu6050Config {
            address: MPU6050_ADDRESS_SECONDARY,
            power_management_1: 0x01,
            config: 0x05,
            gyro_config: 0x08,
            accel_config: 0x10,
        };
        let bus = RecordingI2c::with_register(REG_WHO_AM_I, &[WHO_AM_I_MPU6050]);
        bus.set_register(REG_ACCEL_XOUT_H, &sample_frame());
        let mut sensor = Mpu6050Sensor::new_with_config(bus.clone(), config);

        let _ = sensor.read_imu().unwrap();

        assert_eq!(sensor.address(), MPU6050_ADDRESS_SECONDARY);
        assert_eq!(
            bus.writes(),
            vec![
                (MPU6050_ADDRESS_SECONDARY, vec![REG_PWR_MGMT_1, 0x01]),
                (MPU6050_ADDRESS_SECONDARY, vec![REG_CONFIG, 0x05]),
                (MPU6050_ADDRESS_SECONDARY, vec![REG_GYRO_CONFIG, 0x08]),
                (MPU6050_ADDRESS_SECONDARY, vec![REG_ACCEL_CONFIG, 0x10]),
            ]
        );
    }

    #[test]
    fn mpu6050_sensor_uses_correct_scale_for_non_default_accel_config() {
        // accel_config=0x08 → ±4g → sensitivity=8192 LSB/g
        // raw accel_x = 0x4000 = 16384 → 16384 * 1000 / 8192 = 2000 mg
        let config = Mpu6050Config {
            accel_config: 0x08,
            ..Mpu6050Config::default()
        };
        let bus = RecordingI2c::with_register(REG_WHO_AM_I, &[WHO_AM_I_MPU6050]);
        bus.set_register(REG_ACCEL_XOUT_H, &sample_frame());
        let mut sensor = Mpu6050Sensor::new_with_config(bus, config);

        let reading = sensor.read_imu().unwrap();

        assert_eq!(reading.accel_mg[0], 2000);
    }
}
