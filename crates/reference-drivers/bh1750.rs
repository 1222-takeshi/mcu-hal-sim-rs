//! BH1750 デジタル照度センサドライバ (I2C)
//!
//! アドレス: `0x23` (ADDR=Low) または `0x5C` (ADDR=High)
//! 計測モード: Continuous H-Resolution Mode (1 lx 分解能)

use hal_api::error::{I2cError, SensorError};
use hal_api::i2c::I2cBus;
use hal_api::light::{LightReading, LightSensor};

pub const BH1750_ADDRESS_LOW: u8 = 0x23;
pub const BH1750_ADDRESS_HIGH: u8 = 0x5C;

/// Continuous High-Resolution Mode (120ms 測定周期、0.5 lx 分解能)
const CMD_CONT_H_RES: u8 = 0x10;
/// Power-On
const CMD_POWER_ON: u8 = 0x01;

pub struct Bh1750Sensor<I2C> {
    i2c: I2C,
    address: u8,
}

impl<I2C: I2cBus<Error = I2cError>> Bh1750Sensor<I2C> {
    pub fn new(i2c: I2C, address: u8) -> Result<Self, SensorError> {
        let mut sensor = Self { i2c, address };
        sensor.power_on()?;
        Ok(sensor)
    }

    fn power_on(&mut self) -> Result<(), SensorError> {
        self.i2c
            .write(self.address, &[CMD_POWER_ON])
            .map_err(|_| SensorError::BusError)?;
        self.i2c
            .write(self.address, &[CMD_CONT_H_RES])
            .map_err(|_| SensorError::BusError)
    }
}

impl<I2C: I2cBus<Error = I2cError>> LightSensor for Bh1750Sensor<I2C> {
    type Error = SensorError;

    /// 照度を読み取ります。
    ///
    /// BH1750 は 2 バイトの値を返し、`raw / 1.2` が lux です。
    /// ここでは `raw * 100 / 120` で lux×100 を計算します（整数のみ）。
    fn read_lux(&mut self) -> Result<LightReading, SensorError> {
        let mut buf = [0u8; 2];
        self.i2c
            .read(self.address, &mut buf)
            .map_err(|_| SensorError::BusError)?;
        let raw = u16::from_be_bytes(buf) as u32;
        // lux = raw / 1.2  →  lux×100 = raw * 100 / 1.2 = raw * 500 / 6
        let lux_x100 = raw * 500 / 6;
        Ok(LightReading::new(lux_x100))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hal_api::error::I2cError;
    use hal_api::i2c::I2cBus;

    /// 正常系用 stub: write は成功、read は固定 raw 値を返す
    struct StubI2c {
        raw_high: u8,
        raw_low: u8,
        writes: usize,
    }

    impl StubI2c {
        fn new(raw: u16) -> Self {
            Self {
                raw_high: (raw >> 8) as u8,
                raw_low: raw as u8,
                writes: 0,
            }
        }
    }

    impl I2cBus for StubI2c {
        type Error = I2cError;
        fn write(&mut self, _addr: u8, _data: &[u8]) -> Result<(), I2cError> {
            self.writes += 1;
            Ok(())
        }
        fn read(&mut self, _addr: u8, buf: &mut [u8]) -> Result<(), I2cError> {
            buf[0] = self.raw_high;
            buf[1] = self.raw_low;
            Ok(())
        }
        fn write_read(&mut self, _addr: u8, _write: &[u8], buf: &mut [u8]) -> Result<(), I2cError> {
            buf[0] = self.raw_high;
            buf[1] = self.raw_low;
            Ok(())
        }
    }

    /// 異常系用 stub: write または read で即エラーを返す
    struct FailingI2c {
        fail_write: bool,
        fail_read: bool,
        error: I2cError,
    }

    impl FailingI2c {
        fn fail_on_write(error: I2cError) -> Self {
            Self {
                fail_write: true,
                fail_read: false,
                error,
            }
        }

        fn fail_on_read(error: I2cError) -> Self {
            Self {
                fail_write: false,
                fail_read: true,
                error,
            }
        }
    }

    impl I2cBus for FailingI2c {
        type Error = I2cError;
        fn write(&mut self, _addr: u8, _data: &[u8]) -> Result<(), I2cError> {
            if self.fail_write {
                Err(self.error.clone())
            } else {
                Ok(())
            }
        }
        fn read(&mut self, _addr: u8, _buf: &mut [u8]) -> Result<(), I2cError> {
            if self.fail_read {
                Err(self.error.clone())
            } else {
                Ok(())
            }
        }
        fn write_read(
            &mut self,
            _addr: u8,
            _write: &[u8],
            _buf: &mut [u8],
        ) -> Result<(), I2cError> {
            Err(self.error.clone())
        }
    }

    #[test]
    fn bh1750_converts_raw_to_lux_x100() {
        // raw=1200 → lux = 1200/1.2 = 1000 → lux×100 = 100000
        let i2c = StubI2c::new(1200);
        let mut sensor = Bh1750Sensor::new(i2c, BH1750_ADDRESS_LOW).unwrap();
        let reading = sensor.read_lux().unwrap();
        assert_eq!(reading.lux_integer(), 1000);
    }

    #[test]
    fn bh1750_returns_zero_for_dark() {
        let i2c = StubI2c::new(0);
        let mut sensor = Bh1750Sensor::new(i2c, BH1750_ADDRESS_LOW).unwrap();
        let reading = sensor.read_lux().unwrap();
        assert_eq!(reading.lux_x100, 0);
    }

    #[test]
    fn bh1750_new_maps_write_error_to_bus_error() {
        // power_on 内の write が失敗 → SensorError::BusError として伝播
        let i2c = FailingI2c::fail_on_write(I2cError::BusError);
        match Bh1750Sensor::new(i2c, BH1750_ADDRESS_LOW) {
            Err(e) => assert_eq!(e, SensorError::BusError),
            Ok(_) => panic!("expected Err, got Ok"),
        }
    }

    #[test]
    fn bh1750_read_lux_maps_read_error_to_bus_error() {
        // 初期化は成功させ、read_lux 時の read が失敗 → SensorError::BusError
        let i2c = FailingI2c::fail_on_read(I2cError::Timeout);
        let result = Bh1750Sensor::new(i2c, BH1750_ADDRESS_LOW);
        assert!(result.is_ok(), "new should succeed when write passes");
        let mut sensor = result.unwrap();
        assert_eq!(sensor.read_lux(), Err(SensorError::BusError));
    }

    #[test]
    fn bh1750_new_uses_high_address() {
        let i2c = StubI2c::new(600);
        let mut sensor = Bh1750Sensor::new(i2c, BH1750_ADDRESS_HIGH).unwrap();
        // アドレスが高い方でも正常に計測できること
        let reading = sensor.read_lux().unwrap();
        assert_eq!(reading.lux_integer(), 500);
    }
}
