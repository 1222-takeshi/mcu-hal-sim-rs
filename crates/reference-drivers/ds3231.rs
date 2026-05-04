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

/// DS3231 の時刻レジスタ (0x02) を 24h 制式の 0–23 に変換します。
///
/// bit6 = 12h/24h モードフラグ (1 = 12h モード)
/// bit5 = AM/PM フラグ (12h モード時のみ有効, 1 = PM)
fn decode_hour(raw: u8) -> u8 {
    let is_12h = (raw & 0x40) != 0;
    if is_12h {
        let is_pm = (raw & 0x20) != 0;
        let h = bcd_to_dec(raw & 0x1F); // 1–12
        match (is_pm, h) {
            (false, 12) => 0,    // 12 AM (midnight) = 0:00
            (false, h) => h,     // 1–11 AM
            (true, 12) => 12,    // 12 PM (noon)     = 12:00
            (true, h) => h + 12, // 1–11 PM          = 13–23
        }
    } else {
        bcd_to_dec(raw & 0x3F)
    }
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
        let hour = decode_hour(buf[2]);
        // buf[3] = day of week (1-7)、使用しない
        let day = bcd_to_dec(buf[4] & 0x3F);
        // bit7 = Century bit (DS3231): 2099→2100 で 1 になる。
        // 現実装は 2000–2099 のみ対応。2100 年以降は year() が不正確になる。
        let month = bcd_to_dec(buf[5] & 0x1F);
        let year_offset = bcd_to_dec(buf[6]);

        Ok(RtcDateTime::new(
            year_offset,
            month,
            day,
            hour,
            minute,
            second,
        ))
    }

    fn set_datetime(&mut self, dt: &RtcDateTime) -> Result<(), SensorError> {
        fn dec_to_bcd(dec: u8) -> u8 {
            ((dec / 10) << 4) | (dec % 10)
        }
        let buf = [
            0x00u8, // レジスタポインタ
            dec_to_bcd(dt.second),
            dec_to_bcd(dt.minute),
            dec_to_bcd(dt.hour), // 24h モードで書き込み
            0x01u8,              // day of week (固定: 1)
            dec_to_bcd(dt.day),
            dec_to_bcd(dt.month),
            dec_to_bcd(dt.year_offset),
        ];
        self.i2c
            .write(self.address, &buf)
            .map_err(|_| SensorError::BusError)
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
        fn write_read(&mut self, _addr: u8, _write: &[u8], buf: &mut [u8]) -> Result<(), I2cError> {
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

    #[test]
    fn ds3231_12h_pm_is_converted_to_24h() {
        // 9 PM in 12h mode: bit6=1(12h), bit5=1(PM), BCD=09
        // 0b0110_1001 = 0x69
        let i2c = StubI2c::new(0x00, 0x00, 0x69, 0x01, 0x01, 0x01, 0x25);
        let mut sensor = Ds3231Sensor::new(i2c, DS3231_ADDRESS);
        let dt = sensor.read_datetime().unwrap();
        assert_eq!(dt.hour, 21);
    }

    #[test]
    fn ds3231_12h_am_is_converted_to_24h() {
        // 12 AM (midnight) in 12h mode: bit6=1, bit5=0(AM), BCD=12
        // 0b0101_0010 = 0x52
        let i2c = StubI2c::new(0x00, 0x00, 0x52, 0x01, 0x01, 0x01, 0x25);
        let mut sensor = Ds3231Sensor::new(i2c, DS3231_ADDRESS);
        let dt = sensor.read_datetime().unwrap();
        assert_eq!(dt.hour, 0);
    }

    #[test]
    fn ds3231_propagates_bus_error() {
        struct FailI2c;
        impl I2cBus for FailI2c {
            type Error = I2cError;
            fn write(&mut self, _: u8, _: &[u8]) -> Result<(), I2cError> {
                Ok(())
            }
            fn read(&mut self, _: u8, _: &mut [u8]) -> Result<(), I2cError> {
                Ok(())
            }
            fn write_read(&mut self, _: u8, _: &[u8], _: &mut [u8]) -> Result<(), I2cError> {
                Err(I2cError::BusError)
            }
        }
        let mut sensor = Ds3231Sensor::new(FailI2c, DS3231_ADDRESS);
        assert_eq!(sensor.read_datetime(), Err(SensorError::BusError));
    }

    #[test]
    fn decode_hour_24h_mode() {
        assert_eq!(decode_hour(0x00), 0); // 00:xx
        assert_eq!(decode_hour(0x23), 23); // 23:xx (BCD 0x23 = 23)
        assert_eq!(decode_hour(0x12), 12); // 12:xx (BCD 0x12 = 12)
    }

    #[test]
    fn decode_hour_12h_mode_pm() {
        // 1 PM: bit6=1, bit5=1, BCD=01 → 0x61
        assert_eq!(decode_hour(0x61), 13);
        // 11 PM: bit6=1, bit5=1, BCD=11 → 0x71
        assert_eq!(decode_hour(0x71), 23);
        // 12 PM: bit6=1, bit5=1, BCD=12 → 0x72
        assert_eq!(decode_hour(0x72), 12);
    }

    #[test]
    fn decode_hour_12h_mode_am() {
        // 1 AM: bit6=1, bit5=0, BCD=01 → 0x41
        assert_eq!(decode_hour(0x41), 1);
        // 11 AM: bit6=1, bit5=0, BCD=11 → 0x51
        assert_eq!(decode_hour(0x51), 11);
        // 12 AM: bit6=1, bit5=0, BCD=12 → 0x52
        assert_eq!(decode_hour(0x52), 0);
    }

    #[test]
    fn ds3231_set_datetime_writes_correct_bcd() {
        use hal_api::rtc::RtcSensor;
        struct RecordingI2c {
            last_write: [u8; 8],
        }
        impl RecordingI2c {
            fn new() -> Self {
                Self {
                    last_write: [0u8; 8],
                }
            }
        }
        impl I2cBus for RecordingI2c {
            type Error = I2cError;
            fn write(&mut self, _addr: u8, data: &[u8]) -> Result<(), I2cError> {
                self.last_write[..data.len()].copy_from_slice(data);
                Ok(())
            }
            fn read(&mut self, _: u8, _: &mut [u8]) -> Result<(), I2cError> {
                Ok(())
            }
            fn write_read(&mut self, _: u8, _: &[u8], _: &mut [u8]) -> Result<(), I2cError> {
                Ok(())
            }
        }
        let i2c = RecordingI2c::new();
        let mut sensor = Ds3231Sensor::new(i2c, DS3231_ADDRESS);
        let dt = hal_api::rtc::RtcDateTime::new(25, 5, 4, 12, 30, 45);
        sensor.set_datetime(&dt).unwrap();
        // reg_ptr=0x00, sec=0x45, min=0x30, hour=0x12, dow=0x01, day=0x04, month=0x05, year=0x25
        assert_eq!(sensor.i2c.last_write[0], 0x00); // register pointer
        assert_eq!(sensor.i2c.last_write[1], 0x45); // second BCD
        assert_eq!(sensor.i2c.last_write[2], 0x30); // minute BCD
        assert_eq!(sensor.i2c.last_write[3], 0x12); // hour BCD (24h)
        assert_eq!(sensor.i2c.last_write[5], 0x04); // day BCD
        assert_eq!(sensor.i2c.last_write[6], 0x05); // month BCD
        assert_eq!(sensor.i2c.last_write[7], 0x25); // year BCD
    }
}
