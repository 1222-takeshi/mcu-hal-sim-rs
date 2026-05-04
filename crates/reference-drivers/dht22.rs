//! DHT22 (AM2302) 温湿度センサドライバ (1-wire GPIO)
//!
//! GPIO 1 本で通信するシングルワイヤプロトコルを使用します。
//! ハードウェア側の timing 依存部は [`Dht22RawDevice`] トレイトに委譲し、
//! このドライバはチェックサム検証とデータ抽出を担当します。

use hal_api::error::SensorError;
use hal_api::sensor::{EnvReading, EnvSensor};

/// DHT22 の 1-wire 通信低レベル抽象。
///
/// 実装側は開始パルスの送出と 40bit データの受信を担当します。
pub trait Dht22RawDevice {
    type Error;
    /// 40 ビットの raw data を読み取ります（5 バイトの DHT22 プロトコル）。
    fn read_raw_bytes(&mut self) -> Result<[u8; 5], Self::Error>;
}

#[derive(Debug, Clone)]
pub struct Dht22Sensor<DEV> {
    device: DEV,
}

impl<DEV: Dht22RawDevice> Dht22Sensor<DEV> {
    pub fn new(device: DEV) -> Self {
        Self { device }
    }
}

impl<DEV: Dht22RawDevice<Error = SensorError>> EnvSensor for Dht22Sensor<DEV> {
    type Error = SensorError;

    fn read(&mut self) -> Result<EnvReading, SensorError> {
        let raw = self.device.read_raw_bytes()?;
        // チェックサム検証
        let checksum = (raw[0] as u16 + raw[1] as u16 + raw[2] as u16 + raw[3] as u16) as u8;
        if checksum != raw[4] {
            return Err(SensorError::InvalidReading);
        }
        // 湿度: bytes[0..2]、上位ビット×0.1 %RH
        let hum_raw = u16::from_be_bytes([raw[0], raw[1]]) as u32;
        let humidity_centi_percent = hum_raw * 10; // 0.1%RH → 0.01%RH×100

        // 温度: bytes[2..4]、符号ビットあり
        let temp_raw = u16::from_be_bytes([raw[2] & 0x7F, raw[3]]) as i32;
        let sign = if raw[2] & 0x80 != 0 { -1 } else { 1 };
        let temperature_centi_celsius = sign * temp_raw * 10; // 0.1°C → 0.01°C×100

        Ok(EnvReading::new(
            temperature_centi_celsius,
            humidity_centi_percent,
            None,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubDht22 {
        bytes: [u8; 5],
    }

    impl StubDht22 {
        fn from_reading(hum_x10: u16, temp_x10: i16) -> Self {
            let [h0, h1] = hum_x10.to_be_bytes();
            let temp_raw = temp_x10.unsigned_abs();
            let sign_bit: u8 = if temp_x10 < 0 { 0x80 } else { 0x00 };
            let [t0, t1] = temp_raw.to_be_bytes();
            let t0 = t0 | sign_bit;
            let checksum = (h0 as u16 + h1 as u16 + t0 as u16 + t1 as u16) as u8;
            Self {
                bytes: [h0, h1, t0, t1, checksum],
            }
        }
    }

    impl Dht22RawDevice for StubDht22 {
        type Error = SensorError;
        fn read_raw_bytes(&mut self) -> Result<[u8; 5], SensorError> {
            Ok(self.bytes)
        }
    }

    #[test]
    fn dht22_decodes_positive_temperature() {
        // 25.6°C, 62.3%RH
        let stub = StubDht22::from_reading(623, 256);
        let mut sensor = Dht22Sensor::new(stub);
        let r = sensor.read().unwrap();
        // 256 * 10 = 2560 centi-celsius → 25.60°C
        assert_eq!(r.temperature_centi_celsius, 2560);
        // 623 * 10 = 6230 centi-percent → 62.30%
        assert_eq!(r.humidity_centi_percent, 6230);
    }

    #[test]
    fn dht22_decodes_negative_temperature() {
        // -5.0°C, 40.0%RH
        let stub = StubDht22::from_reading(400, -50);
        let mut sensor = Dht22Sensor::new(stub);
        let r = sensor.read().unwrap();
        assert_eq!(r.temperature_centi_celsius, -500);
        assert_eq!(r.humidity_centi_percent, 4000);
    }

    #[test]
    fn dht22_checksum_error_returns_invalid_data() {
        let mut stub = StubDht22::from_reading(623, 256);
        stub.bytes[4] = 0xFF; // corrupt checksum
        let mut sensor = Dht22Sensor::new(stub);
        assert!(matches!(sensor.read(), Err(SensorError::InvalidReading)));
    }
}
