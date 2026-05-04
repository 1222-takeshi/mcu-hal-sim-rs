//! SGP30 CO₂ / VOC センサドライバ (I2C)
//!
//! アドレス: `0x58`（固定）
//!
//! ## プロトコル概要
//! SGP30 は 2 バイトコマンドを書いてから 6 バイトを読み取るプロトコルを使用します。
//!
//! | コマンド      | バイト          | 意味                  |
//! |--------------|-----------------|----------------------|
//! | 0x20, 0x03   | init_air_quality | センサを初期化        |
//! | 0x20, 0x08   | measure_air_quality | CO₂+VOC を測定    |
//!
//! 測定結果: `[CO2_H, CO2_L, CRC, VOC_H, VOC_L, CRC]` (各 CRC は無視可)

use hal_api::error::{I2cError, SensorError};
use hal_api::gas::{GasReading, GasSensor};
use hal_api::i2c::I2cBus;

pub const SGP30_ADDRESS: u8 = 0x58;

const CMD_INIT: [u8; 2] = [0x20, 0x03];
const CMD_MEASURE: [u8; 2] = [0x20, 0x08];

/// SGP30 ドライバ。
pub struct Sgp30Sensor<I2C> {
    i2c: I2C,
    address: u8,
}

impl<I2C: I2cBus<Error = I2cError>> Sgp30Sensor<I2C> {
    /// 新しい SGP30 ドライバを作成し、`init_air_quality` コマンドを送信します。
    pub fn new(i2c: I2C, address: u8) -> Result<Self, SensorError> {
        let mut s = Self { i2c, address };
        s.i2c
            .write(s.address, &CMD_INIT)
            .map_err(|_| SensorError::BusError)?;
        Ok(s)
    }
}

impl<I2C: I2cBus<Error = I2cError>> GasSensor for Sgp30Sensor<I2C> {
    type Error = SensorError;

    /// CO₂ / VOC を読み取ります。
    ///
    /// `measure_air_quality` コマンドを送信し、6 バイトの応答から CO₂ と VOC を取得します。
    /// CRC バイト（offset 2, 5）は省略します。
    fn read_gas(&mut self) -> Result<GasReading, SensorError> {
        self.i2c
            .write(self.address, &CMD_MEASURE)
            .map_err(|_| SensorError::BusError)?;
        let mut buf = [0u8; 6];
        self.i2c
            .read(self.address, &mut buf)
            .map_err(|_| SensorError::BusError)?;
        // CRC バイト (buf[2], buf[5]) は SGP30 CRC-8 (多項式 0x31) で保護されているが、
        // このドライバでは検証を省略しています。信頼性が必要な場合は CRC を検証してください。
        let co2_ppm = u16::from_be_bytes([buf[0], buf[1]]);
        let voc_ppb = u16::from_be_bytes([buf[3], buf[4]]);
        Ok(GasReading::new(co2_ppm, voc_ppb))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hal_api::i2c::I2cBus;

    struct StubI2c {
        co2_h: u8,
        co2_l: u8,
        voc_h: u8,
        voc_l: u8,
        writes: usize,
    }

    impl StubI2c {
        fn new(co2: u16, voc: u16) -> Self {
            Self {
                co2_h: (co2 >> 8) as u8,
                co2_l: co2 as u8,
                voc_h: (voc >> 8) as u8,
                voc_l: voc as u8,
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
            // [CO2_H, CO2_L, CRC=0, VOC_H, VOC_L, CRC=0]
            buf[0] = self.co2_h;
            buf[1] = self.co2_l;
            buf[2] = 0x00; // CRC placeholder
            buf[3] = self.voc_h;
            buf[4] = self.voc_l;
            buf[5] = 0x00; // CRC placeholder
            Ok(())
        }
        fn write_read(
            &mut self,
            _addr: u8,
            _write: &[u8],
            _buf: &mut [u8],
        ) -> Result<(), I2cError> {
            Ok(())
        }
    }

    #[test]
    fn sgp30_reads_co2_and_voc() {
        let i2c = StubI2c::new(400, 0);
        let mut sensor = Sgp30Sensor::new(i2c, SGP30_ADDRESS).unwrap();
        let reading = sensor.read_gas().unwrap();
        assert_eq!(reading.co2_ppm, 400);
        assert_eq!(reading.voc_ppb, 0);
    }

    #[test]
    fn sgp30_reads_elevated_co2_and_voc() {
        let i2c = StubI2c::new(1200, 350);
        let mut sensor = Sgp30Sensor::new(i2c, SGP30_ADDRESS).unwrap();
        let reading = sensor.read_gas().unwrap();
        assert_eq!(reading.co2_ppm, 1200);
        assert_eq!(reading.voc_ppb, 350);
    }

    #[test]
    fn sgp30_init_sends_init_command() {
        let i2c = StubI2c::new(400, 0);
        let sensor = Sgp30Sensor::new(i2c, SGP30_ADDRESS).unwrap();
        assert_eq!(sensor.i2c.writes, 1, "init should send 1 write command");
    }

    #[test]
    fn sgp30_new_fails_on_bus_error() {
        struct FailI2c;
        impl I2cBus for FailI2c {
            type Error = I2cError;
            fn write(&mut self, _: u8, _: &[u8]) -> Result<(), I2cError> {
                Err(I2cError::BusError)
            }
            fn read(&mut self, _: u8, _: &mut [u8]) -> Result<(), I2cError> { Ok(()) }
            fn write_read(&mut self, _: u8, _: &[u8], _: &mut [u8]) -> Result<(), I2cError> { Ok(()) }
        }
        assert!(matches!(Sgp30Sensor::new(FailI2c, SGP30_ADDRESS), Err(SensorError::BusError)));
    }

    #[test]
    fn sgp30_read_gas_fails_on_bus_error() {
        struct FailReadI2c { init_done: bool }
        impl I2cBus for FailReadI2c {
            type Error = I2cError;
            fn write(&mut self, _: u8, _: &[u8]) -> Result<(), I2cError> {
                if self.init_done { Err(I2cError::BusError) } else { self.init_done = true; Ok(()) }
            }
            fn read(&mut self, _: u8, _: &mut [u8]) -> Result<(), I2cError> { Err(I2cError::BusError) }
            fn write_read(&mut self, _: u8, _: &[u8], _: &mut [u8]) -> Result<(), I2cError> { Ok(()) }
        }
        // init succeeds (first write), then measure write fails
        let i2c = FailReadI2c { init_done: false };
        let mut sensor = Sgp30Sensor::new(i2c, SGP30_ADDRESS).unwrap();
        assert!(matches!(sensor.read_gas(), Err(SensorError::BusError)));
    }
}
