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
}
