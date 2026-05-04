//! Host-side DHT22 温湿度センサモック

use hal_api::error::SensorError;
use hal_api::sensor::{EnvReading, EnvSensor};
use reference_drivers::dht22::Dht22RawDevice;
use std::cell::RefCell;
use std::rc::Rc;
use std::vec::Vec;

#[derive(Debug)]
struct MockDht22State {
    readings: Vec<(i16, u16)>, // (temp_x10, humidity_x10) pairs
    next_index: usize,
    loop_forever: bool,
    read_count: usize,
}

#[derive(Clone, Debug)]
pub struct MockDht22Device {
    state: Rc<RefCell<MockDht22State>>,
}

impl MockDht22Device {
    /// (temp×10, humidity×10) ペアのシーケンスを返すモックを生成します。
    ///
    /// 例: `(256, 623)` は 25.6°C, 62.3%RH を表します。
    pub fn new(readings: Vec<(i16, u16)>) -> Self {
        Self {
            state: Rc::new(RefCell::new(MockDht22State {
                readings,
                next_index: 0,
                loop_forever: false,
                read_count: 0,
            })),
        }
    }

    pub fn looping(readings: Vec<(i16, u16)>) -> Self {
        let device = Self::new(readings);
        device.state.borrow_mut().loop_forever = true;
        device
    }

    pub fn fixed(temp_x10: i16, humidity_x10: u16) -> Self {
        Self::looping(vec![(temp_x10, humidity_x10)])
    }

    pub fn read_count(&self) -> usize {
        self.state.borrow().read_count
    }
}

impl Default for MockDht22Device {
    fn default() -> Self {
        // デフォルト: 25.0°C, 60.0%RH
        Self::fixed(250, 600)
    }
}

impl Dht22RawDevice for MockDht22Device {
    type Error = SensorError;

    fn read_raw_bytes(&mut self) -> Result<[u8; 5], SensorError> {
        let mut state = self.state.borrow_mut();
        let &(temp_x10, hum_x10) = state
            .readings
            .get(state.next_index)
            .ok_or(SensorError::NotInitialized)?;

        state.read_count += 1;

        if state.loop_forever {
            state.next_index = (state.next_index + 1) % state.readings.len();
        } else if state.next_index + 1 < state.readings.len() {
            state.next_index += 1;
        }

        let [h0, h1] = hum_x10.to_be_bytes();
        let temp_raw = temp_x10.unsigned_abs();
        let sign_bit: u8 = if temp_x10 < 0 { 0x80 } else { 0x00 };
        let [t0, t1] = temp_raw.to_be_bytes();
        let t0 = t0 | sign_bit;
        let checksum = (h0 as u16 + h1 as u16 + t0 as u16 + t1 as u16) as u8;
        Ok([h0, h1, t0, t1, checksum])
    }
}

/// `EnvSensor` 直接実装（軽量版、VirtualI2cBus 不要）
#[derive(Clone, Debug)]
pub struct MockDht22EnvSensor {
    inner: MockDht22Device,
}

impl MockDht22EnvSensor {
    pub fn fixed(temp_x10: i16, humidity_x10: u16) -> Self {
        Self {
            inner: MockDht22Device::fixed(temp_x10, humidity_x10),
        }
    }
}

impl Default for MockDht22EnvSensor {
    fn default() -> Self {
        Self::fixed(250, 600)
    }
}

impl EnvSensor for MockDht22EnvSensor {
    type Error = SensorError;

    fn read(&mut self) -> Result<EnvReading, SensorError> {
        let raw = self.inner.read_raw_bytes()?;
        let hum_raw = u16::from_be_bytes([raw[0], raw[1]]) as u32;
        let temp_raw = u16::from_be_bytes([raw[2] & 0x7F, raw[3]]) as i32;
        let sign = if raw[2] & 0x80 != 0 { -1 } else { 1 };
        Ok(EnvReading::new(sign * temp_raw * 10, hum_raw * 10, None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_dht22_raw_bytes_valid_checksum() {
        let mut device = MockDht22Device::fixed(256, 623);
        let raw = device.read_raw_bytes().unwrap();
        let expected_checksum =
            (raw[0] as u16 + raw[1] as u16 + raw[2] as u16 + raw[3] as u16) as u8;
        assert_eq!(raw[4], expected_checksum);
    }

    #[test]
    fn mock_dht22_env_sensor_returns_correct_values() {
        let mut sensor = MockDht22EnvSensor::fixed(256, 623); // 25.6°C, 62.3%
        let r = sensor.read().unwrap();
        assert_eq!(r.temperature_centi_celsius, 2560);
        assert_eq!(r.humidity_centi_percent, 6230);
    }

    #[test]
    fn mock_dht22_negative_temperature() {
        let mut sensor = MockDht22EnvSensor::fixed(-50, 400); // -5.0°C, 40.0%
        let r = sensor.read().unwrap();
        assert_eq!(r.temperature_centi_celsius, -500);
        assert_eq!(r.humidity_centi_percent, 4000);
    }
}
