//! BME280 センサドライバ

use hal_api::error::{I2cError, SensorError};
use hal_api::i2c::I2cBus;
use hal_api::sensor::{EnvReading, EnvSensor};

const REG_CHIP_ID: u8 = 0xD0;
const REG_CALIB_1_START: u8 = 0x88;
const REG_CALIB_2_START: u8 = 0xE1;
const REG_CTRL_HUM: u8 = 0xF2;
const REG_STATUS: u8 = 0xF3;
const REG_CTRL_MEAS: u8 = 0xF4;
const REG_CONFIG: u8 = 0xF5;
const REG_PRESS_MSB: u8 = 0xF7;
const CHIP_ID_BME280: u8 = 0x60;
const MAX_STATUS_POLLS: usize = 8;

pub const BME280_ADDRESS_PRIMARY: u8 = 0x77;
pub const BME280_ADDRESS_SECONDARY: u8 = 0x76;

#[derive(Debug, Clone, Copy)]
struct CalibrationData {
    dig_t1: u16,
    dig_t2: i16,
    dig_t3: i16,
    dig_p1: u16,
    dig_p2: i16,
    dig_p3: i16,
    dig_p4: i16,
    dig_p5: i16,
    dig_p6: i16,
    dig_p7: i16,
    dig_p8: i16,
    dig_p9: i16,
    dig_h1: u8,
    dig_h2: i16,
    dig_h3: u8,
    dig_h4: i16,
    dig_h5: i16,
    dig_h6: i8,
}

impl CalibrationData {
    fn from_registers(calib_1: &[u8; 26], calib_2: &[u8; 7]) -> Self {
        Self {
            dig_t1: u16::from_le_bytes([calib_1[0], calib_1[1]]),
            dig_t2: i16::from_le_bytes([calib_1[2], calib_1[3]]),
            dig_t3: i16::from_le_bytes([calib_1[4], calib_1[5]]),
            dig_p1: u16::from_le_bytes([calib_1[6], calib_1[7]]),
            dig_p2: i16::from_le_bytes([calib_1[8], calib_1[9]]),
            dig_p3: i16::from_le_bytes([calib_1[10], calib_1[11]]),
            dig_p4: i16::from_le_bytes([calib_1[12], calib_1[13]]),
            dig_p5: i16::from_le_bytes([calib_1[14], calib_1[15]]),
            dig_p6: i16::from_le_bytes([calib_1[16], calib_1[17]]),
            dig_p7: i16::from_le_bytes([calib_1[18], calib_1[19]]),
            dig_p8: i16::from_le_bytes([calib_1[20], calib_1[21]]),
            dig_p9: i16::from_le_bytes([calib_1[22], calib_1[23]]),
            dig_h1: calib_1[25],
            dig_h2: i16::from_le_bytes([calib_2[0], calib_2[1]]),
            dig_h3: calib_2[2],
            dig_h4: sign_extend_12(((u16::from(calib_2[3])) << 4) | (u16::from(calib_2[4]) & 0x0F)),
            dig_h5: sign_extend_12(((u16::from(calib_2[5])) << 4) | (u16::from(calib_2[4]) >> 4)),
            dig_h6: calib_2[6] as i8,
        }
    }

    fn compensate_temperature(&self, adc_temp: i32) -> (i32, i32) {
        let var1 = (((adc_temp >> 3) - ((self.dig_t1 as i32) << 1)) * self.dig_t2 as i32) >> 11;
        let var2 = (((((adc_temp >> 4) - self.dig_t1 as i32)
            * ((adc_temp >> 4) - self.dig_t1 as i32))
            >> 12)
            * self.dig_t3 as i32)
            >> 14;
        let t_fine = var1 + var2;
        let temperature_centi_celsius = (t_fine * 5 + 128) >> 8;
        (temperature_centi_celsius, t_fine)
    }

    fn compensate_pressure(&self, adc_pressure: i32, t_fine: i32) -> Option<u32> {
        let mut var1 = i64::from(t_fine) - 128_000;
        let mut var2 = var1 * var1 * i64::from(self.dig_p6);
        var2 += (var1 * i64::from(self.dig_p5)) << 17;
        var2 += i64::from(self.dig_p4) << 35;
        var1 =
            ((var1 * var1 * i64::from(self.dig_p3)) >> 8) + ((var1 * i64::from(self.dig_p2)) << 12);
        var1 = ((((1_i64) << 47) + var1) * i64::from(self.dig_p1)) >> 33;
        if var1 == 0 {
            return None;
        }

        let mut pressure = 1_048_576 - i64::from(adc_pressure);
        pressure = (((pressure << 31) - var2) * 3_125) / var1;
        var1 = (i64::from(self.dig_p9) * (pressure >> 13) * (pressure >> 13)) >> 25;
        var2 = (i64::from(self.dig_p8) * pressure) >> 19;
        pressure = ((pressure + var1 + var2) >> 8) + (i64::from(self.dig_p7) << 4);

        Some((pressure / 256) as u32)
    }

    fn compensate_humidity(&self, adc_humidity: i32, t_fine: i32) -> u32 {
        let mut humidity = t_fine - 76_800;
        humidity = ((((adc_humidity << 14)
            - ((self.dig_h4 as i32) << 20)
            - ((self.dig_h5 as i32) * humidity))
            + 16_384)
            >> 15)
            * (((((((humidity * self.dig_h6 as i32) >> 10)
                * (((humidity * self.dig_h3 as i32) >> 11) + 32_768))
                >> 10)
                + 2_097_152)
                * self.dig_h2 as i32
                + 8_192)
                >> 14);
        humidity -= ((((humidity >> 15) * (humidity >> 15)) >> 7) * self.dig_h1 as i32) >> 4;
        humidity = humidity.clamp(0, 419_430_400);

        ((humidity >> 12) * 100 / 1024) as u32
    }
}

fn sign_extend_12(value: u16) -> i16 {
    let extended = if value & 0x800 != 0 {
        value | 0xF000
    } else {
        value
    };
    extended as i16
}

/// BME280 ドライバ。
pub struct Bme280Sensor<B> {
    bus: B,
    address: u8,
    calibration: Option<CalibrationData>,
    last_reading: Option<EnvReading>,
}

impl<B> Bme280Sensor<B> {
    pub fn new(bus: B) -> Self {
        Self::new_with_address(bus, BME280_ADDRESS_PRIMARY)
    }

    pub fn new_with_address(bus: B, address: u8) -> Self {
        Self {
            bus,
            address,
            calibration: None,
            last_reading: None,
        }
    }

    pub fn address(&self) -> u8 {
        self.address
    }

    #[cfg(test)]
    pub fn is_initialized(&self) -> bool {
        self.calibration.is_some()
    }
}

impl<B> Bme280Sensor<B>
where
    B: I2cBus<Error = I2cError>,
{
    fn initialize(&mut self) -> Result<(), SensorError> {
        if self.calibration.is_some() {
            return Ok(());
        }

        let chip_id = self.read_u8(REG_CHIP_ID)?;
        if chip_id != CHIP_ID_BME280 {
            return Err(SensorError::InvalidReading);
        }

        let mut calib_1 = [0u8; 26];
        let mut calib_2 = [0u8; 7];
        self.read_registers(REG_CALIB_1_START, &mut calib_1)?;
        self.read_registers(REG_CALIB_2_START, &mut calib_2)?;

        self.write_register(REG_CTRL_HUM, 0x01)?;
        self.write_register(REG_CTRL_MEAS, 0x27)?;
        self.write_register(REG_CONFIG, 0x00)?;
        self.calibration = Some(CalibrationData::from_registers(&calib_1, &calib_2));
        Ok(())
    }

    fn read_u8(&mut self, register: u8) -> Result<u8, SensorError> {
        let mut value = [0u8; 1];
        self.read_registers(register, &mut value)?;
        Ok(value[0])
    }

    fn read_registers(&mut self, register: u8, buffer: &mut [u8]) -> Result<(), SensorError> {
        self.bus
            .write_read(self.address, &[register], buffer)
            .map_err(map_sensor_error)
    }

    fn write_register(&mut self, register: u8, value: u8) -> Result<(), SensorError> {
        self.bus
            .write(self.address, &[register, value])
            .map_err(map_sensor_error)
    }

    fn read_raw_sample(&mut self) -> Result<(i32, i32, i32), SensorError> {
        let mut raw = [0u8; 8];
        self.read_registers(REG_PRESS_MSB, &mut raw)?;

        let adc_pressure =
            ((i32::from(raw[0])) << 12) | ((i32::from(raw[1])) << 4) | (i32::from(raw[2]) >> 4);
        let adc_temperature =
            ((i32::from(raw[3])) << 12) | ((i32::from(raw[4])) << 4) | (i32::from(raw[5]) >> 4);
        let adc_humidity = ((i32::from(raw[6])) << 8) | i32::from(raw[7]);

        if adc_temperature == 0x80_000 || adc_humidity == 0x80_00 {
            return Err(SensorError::InvalidReading);
        }

        Ok((adc_temperature, adc_pressure, adc_humidity))
    }
}

impl<B> EnvSensor for Bme280Sensor<B>
where
    B: I2cBus<Error = I2cError>,
{
    type Error = SensorError;

    fn read(&mut self) -> Result<EnvReading, Self::Error> {
        self.initialize()?;

        let mut status_busy = false;
        let mut polls_remaining = MAX_STATUS_POLLS;
        while self.read_u8(REG_STATUS)? & 0x08 != 0 {
            status_busy = true;
            polls_remaining -= 1;
            if polls_remaining == 0 {
                break;
            }
        }

        let calibration = self.calibration.ok_or(SensorError::NotInitialized)?;
        let (adc_temperature, adc_pressure, adc_humidity) = match self.read_raw_sample() {
            Ok(sample) => sample,
            Err(SensorError::InvalidReading) if status_busy => {
                return self.last_reading.ok_or(SensorError::Busy)
            }
            Err(error) => return Err(error),
        };
        let (temperature_centi_celsius, t_fine) =
            calibration.compensate_temperature(adc_temperature);
        let pressure_pascal = calibration.compensate_pressure(adc_pressure, t_fine);
        let humidity_centi_percent = calibration.compensate_humidity(adc_humidity, t_fine);
        let reading = EnvReading::new(
            temperature_centi_celsius,
            humidity_centi_percent,
            pressure_pascal,
        );
        self.last_reading = Some(reading);
        Ok(reading)
    }
}

fn map_sensor_error(_error: I2cError) -> SensorError {
    SensorError::BusError
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::RefCell;
    use std::rc::Rc;
    use std::vec::Vec;

    type RegisterResponses = Rc<RefCell<Vec<(u8, Vec<u8>)>>>;

    #[derive(Clone, Default)]
    struct RecordingI2c {
        writes: Rc<RefCell<Vec<Vec<u8>>>>,
        responses: RegisterResponses,
    }

    impl RecordingI2c {
        fn with_bme280_defaults() -> Self {
            let bus = Self::default();
            bus.set_response(REG_CHIP_ID, &[CHIP_ID_BME280]);
            bus.set_response(REG_STATUS, &[0x00]);
            bus.set_response(
                REG_CALIB_1_START,
                &[
                    0x70, 0x6B, 0x43, 0x67, 0x18, 0xFC, 0x7D, 0x8E, 0x43, 0xD6, 0xD0, 0x0B, 0x27,
                    0x0B, 0x8C, 0x00, 0xF9, 0xFF, 0x8C, 0x3C, 0xF8, 0xC6, 0x70, 0x17, 0x00, 0x4B,
                ],
            );
            bus.set_response(
                REG_CALIB_2_START,
                &[0x6A, 0x01, 0x00, 0x14, 0x25, 0x03, 0x1E],
            );
            bus.set_response(
                REG_PRESS_MSB,
                &[0x65, 0x5A, 0xC0, 0x7E, 0xED, 0x00, 0x89, 0x98],
            );
            bus
        }

        fn set_response(&self, register: u8, bytes: &[u8]) {
            let mut responses = self.responses.borrow_mut();
            responses.retain(|(candidate, _)| *candidate != register);
            responses.push((register, bytes.to_vec()));
        }
    }

    impl I2cBus for RecordingI2c {
        type Error = I2cError;

        fn write(&mut self, _addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
            self.writes.borrow_mut().push(bytes.to_vec());
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
            let response = self
                .responses
                .borrow()
                .iter()
                .find(|(candidate, _)| *candidate == register)
                .map(|(_, data)| data.clone())
                .ok_or(I2cError::InvalidAddress)?;
            if response.len() != buffer.len() {
                return Err(I2cError::BusError);
            }
            buffer.copy_from_slice(&response);
            Ok(())
        }
    }

    #[test]
    fn sign_extend_12_handles_positive_and_negative_values() {
        assert_eq!(sign_extend_12(0x07F), 127);
        assert_eq!(sign_extend_12(0xF80), -128);
    }

    #[test]
    fn bme280_sensor_initializes_and_reads_environment() {
        let bus = RecordingI2c::with_bme280_defaults();
        let writes = bus.writes.clone();
        let mut sensor = Bme280Sensor::new(bus);

        let reading = sensor.read().unwrap();

        assert!(sensor.is_initialized());
        assert!(reading.temperature_centi_celsius > 0);
        assert!(reading.humidity_centi_percent <= 10_000);
        assert!(reading.pressure_pascal.unwrap_or_default() > 50_000);
        assert!(writes
            .borrow()
            .iter()
            .any(|bytes| bytes.as_slice() == [REG_CTRL_HUM, 0x01]));
        assert!(writes
            .borrow()
            .iter()
            .any(|bytes| bytes.as_slice() == [REG_CTRL_MEAS, 0x27]));
    }

    #[test]
    fn bme280_sensor_rejects_unexpected_chip_id() {
        let bus = RecordingI2c::default();
        bus.set_response(REG_CHIP_ID, &[0x00]);
        let mut sensor = Bme280Sensor::new(bus);

        assert_eq!(sensor.read(), Err(SensorError::InvalidReading));
    }

    #[test]
    fn bme280_sensor_reads_last_completed_sample_while_measuring() {
        let bus = RecordingI2c::with_bme280_defaults();
        bus.set_response(REG_STATUS, &[0x08]);
        let mut sensor = Bme280Sensor::new(bus);

        let reading = sensor.read().unwrap();

        assert!(reading.temperature_centi_celsius > 0);
        assert!(reading.humidity_centi_percent <= 10_000);
    }

    #[test]
    fn bme280_sensor_rejects_invalid_raw_sample_markers() {
        let bus = RecordingI2c::with_bme280_defaults();
        bus.set_response(
            REG_PRESS_MSB,
            &[0x65, 0x5A, 0xC0, 0x80, 0x00, 0x00, 0x80, 0x00],
        );
        let mut sensor = Bme280Sensor::new(bus);

        assert_eq!(sensor.read(), Err(SensorError::InvalidReading));
    }

    #[test]
    fn bme280_sensor_reports_busy_when_measuring_and_no_last_sample_exists() {
        let bus = RecordingI2c::with_bme280_defaults();
        bus.set_response(REG_STATUS, &[0x08]);
        bus.set_response(
            REG_PRESS_MSB,
            &[0x65, 0x5A, 0xC0, 0x80, 0x00, 0x00, 0x80, 0x00],
        );
        let mut sensor = Bme280Sensor::new(bus);

        assert_eq!(sensor.read(), Err(SensorError::Busy));
    }
}
