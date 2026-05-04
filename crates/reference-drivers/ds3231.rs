//! DS3231 高精度 RTC ドライバ (I2C)
//!
//! アドレス: `0x68`
//!
//! **注意**: DS3231 の I2C アドレス (0x68) は MPU6050 と同一です。
//! 同一バス上での同時使用はできません。どちらか一方を選択してください。
//!
//! ## レジスタマップ (読み取り)
//! | offset | 内容 |
//! |--------|------|
//! | 0x00   | Seconds (BCD) |
//! | 0x01   | Minutes (BCD) |
//! | 0x02   | Hours (BCD, bit6=12/24h) |
//! | 0x03   | Day of week |
//! | 0x04   | Date (BCD) |
//! | 0x05   | Month (BCD) |
//! | 0x06   | Year (BCD) |

use hal_api::error::{I2cError, SensorError};
use hal_api::i2c::I2cBus;
use hal_api::rtc::{RtcDateTime, RtcSensor};

pub const DS3231_ADDRESS: u8 = 0x68;

/// DS3231 ドライバ。
pub struct Ds3231Sensor<I2C> {
    i2c: I2C,
    address: u8,
}

impl<I2C: I2cBus<Error = I2cError>> Ds3231Sensor<I2C> {
    pub fn new(i2c: I2C, address: u8) -> Self {
        Self { i2c, address }
    }
}

/// BCD 値を 10 進数に変換します。
fn bcd_to_dec(bcd: u8) -> u8 {
    (bcd >> 4) * 10 + (bcd & 0x0F)
}

impl<I2C: I2cBus<Error = I2cError>> RtcSensor for Ds3231Sensor<I2C> {
    type Error = SensorError;

    /// 現在の日時を読み取ります。
    ///
    /// レジスタ 0x00 から 7 バイトを読み取り、BCD → 10 進変換して返します。
    fn read_datetime(&mut self) -> Result<RtcDateTime, SensorError> {
        let mut buf = [0u8; 7];
        self.i2c
            .write_read(self.address, &[0x00], &mut buf)
            .map_err(|_| SensorError::BusError)?;

        let second = bcd_to_dec(buf[0] & 0x7F);
        let minute = bcd_to_dec(buf[1] & 0x7F);
        // bit6 = 12/24h モードフラグ。ここでは 24h として扱う
        let hour = bcd_to_dec(buf[2] & 0x3F);
        // buf[3] = day of week (1-7)、使用しない
        let day = bcd_to_dec(buf[4] & 0x3F);
        let month = bcd_to_dec(buf[5] & 0x1F);
        let year_offset = bcd_to_dec(buf[6]);

        Ok(RtcDateTime::new(year_offset, month, day, hour, minute, second))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hal_api::i2c::I2cBus;

    struct StubI2c {
        /// 7バイトのレジスタ値（BCD形式）
        regs: [u8; 7],
    }

    impl StubI2c {
        fn new(sec: u8, min: u8, hour: u8, dow: u8, day: u8, month: u8, year: u8) -> Self {
            Self {
                regs: [sec, min, hour, dow, day, month, year],
            }
        }
    }

    impl I2cBus for StubI2c {
        type Error = I2cError;
        fn write(&mut self, _addr: u8, _data: &[u8]) -> Result<(), I2cError> {
            Ok(())
        }
        fn read(&mut self, _addr: u8, buf: &mut [u8]) -> Result<(), I2cError> {
            buf.copy_from_slice(&self.regs[..buf.len()]);
            Ok(())
        }
        fn write_read(
            &mut self,
            _addr: u8,
            _write: &[u8],
            buf: &mut [u8],
        ) -> Result<(), I2cError> {
            buf.copy_from_slice(&self.regs[..buf.len()]);
            Ok(())
        }
    }

    #[test]
    fn ds3231_reads_datetime_correctly() {
        // BCD values: 2025-05-04 12:30:45
        // year=0x25, month=0x05, day=0x04, hour=0x12, min=0x30, sec=0x45
        let i2c = StubI2c::new(0x45, 0x30, 0x12, 0x01, 0x04, 0x05, 0x25);
        let mut sensor = Ds3231Sensor::new(i2c, DS3231_ADDRESS);
        let dt = sensor.read_datetime().unwrap();
        assert_eq!(dt.year(), 2025);
        assert_eq!(dt.month, 5);
        assert_eq!(dt.day, 4);
        assert_eq!(dt.hour, 12);
        assert_eq!(dt.minute, 30);
        assert_eq!(dt.second, 45);
    }

    #[test]
    fn ds3231_midnight_is_decoded_correctly() {
        // 00:00:00 → BCD all zeros
        let i2c = StubI2c::new(0x00, 0x00, 0x00, 0x01, 0x01, 0x01, 0x00);
        let mut sensor = Ds3231Sensor::new(i2c, DS3231_ADDRESS);
        let dt = sensor.read_datetime().unwrap();
        assert_eq!(dt.hour, 0);
        assert_eq!(dt.minute, 0);
        assert_eq!(dt.second, 0);
        assert_eq!(dt.year(), 2000);
    }

    #[test]
    fn bcd_to_dec_converts_correctly() {
        assert_eq!(bcd_to_dec(0x00), 0);
        assert_eq!(bcd_to_dec(0x09), 9);
        assert_eq!(bcd_to_dec(0x10), 10);
        assert_eq!(bcd_to_dec(0x59), 59);
        assert_eq!(bcd_to_dec(0x99), 99);
    }
}
