//! VL53L0X Time-of-Flight 距離センサドライバ (I2C)
//!
//! アドレス: `0x29`（デフォルト）
//!
//! ## プロトコル概要（簡略版）
//! VL53L0X の完全な初期化シーケンスは複雑ですが、
//! このドライバはシミュレーション・リファレンス用として主要動作のみを実装します。
//!
//! | 操作 | レジスタ | 説明 |
//! |------|---------|------|
//! | 初期化 | 0x80 = 0x01, 0xFF = 0x01, 0x00 = 0x00 | センサ有効化 |
//! | 測定開始 | SYSRANGE_START (0x00) = 0x01 | 1 shot 計測 |
//! | 結果読み取り | RESULT_RANGE_MM (0x1E) 2バイト | 距離 (mm) |

use hal_api::distance::{DistanceReading, DistanceSensor};
use hal_api::error::{I2cError, SensorError};
use hal_api::i2c::I2cBus;

pub const VL53L0X_ADDRESS: u8 = 0x29;

const REG_IDENTIFICATION_MODEL_ID: u8 = 0xC0;
const EXPECTED_MODEL_ID: u8 = 0xEE;
const REG_SYSRANGE_START: u8 = 0x00;
const REG_RESULT_RANGE_MM: u8 = 0x1E;
const REG_RESULT_INTERRUPT_STATUS: u8 = 0x13;
const MAX_RANGE_POLLS: usize = 10;

/// VL53L0X ドライバ。
pub struct Vl53l0xSensor<I2C> {
    i2c: I2C,
    address: u8,
}

impl<I2C: I2cBus<Error = I2cError>> Vl53l0xSensor<I2C> {
    /// 新しい VL53L0X ドライバを作成します。
    ///
    /// モデル ID レジスタ (0xC0) を確認して正しいデバイスかを検証します。
    pub fn new(i2c: I2C, address: u8) -> Result<Self, SensorError> {
        let mut s = Self { i2c, address };
        s.verify_identity()?;
        Ok(s)
    }

    fn verify_identity(&mut self) -> Result<(), SensorError> {
        let mut buf = [0u8; 1];
        self.i2c
            .write_read(self.address, &[REG_IDENTIFICATION_MODEL_ID], &mut buf)
            .map_err(|_| SensorError::BusError)?;
        if buf[0] != EXPECTED_MODEL_ID {
            return Err(SensorError::InvalidReading);
        }
        Ok(())
    }
}

impl<I2C: I2cBus<Error = I2cError>> DistanceSensor for Vl53l0xSensor<I2C> {
    type Error = SensorError;

    /// ToF で距離を測定します（mm）。
    ///
    /// 1-shot 計測コマンドを送り、計測完了を待ってから 2 バイトの距離結果を読み取ります。
    fn read_distance(&mut self) -> Result<DistanceReading, SensorError> {
        // 1-shot 計測開始
        self.i2c
            .write(self.address, &[REG_SYSRANGE_START, 0x01])
            .map_err(|_| SensorError::BusError)?;
        // 計測完了待ち: RESULT_INTERRUPT_STATUS (0x13) の bit[2:0] != 0 で完了
        let mut ready = false;
        for _ in 0..MAX_RANGE_POLLS {
            let mut status = [0u8; 1];
            self.i2c
                .write_read(self.address, &[REG_RESULT_INTERRUPT_STATUS], &mut status)
                .map_err(|_| SensorError::BusError)?;
            if status[0] & 0x07 != 0 {
                ready = true;
                break;
            }
        }
        if !ready {
            return Err(SensorError::Busy);
        }
        // 距離結果レジスタを読み取る
        let mut buf = [0u8; 2];
        self.i2c
            .write_read(self.address, &[REG_RESULT_RANGE_MM], &mut buf)
            .map_err(|_| SensorError::BusError)?;
        let distance_mm = u16::from_be_bytes(buf) as u32;
        Ok(DistanceReading::new(distance_mm))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hal_api::i2c::I2cBus;

    struct StubI2c {
        distance_h: u8,
        distance_l: u8,
    }

    impl StubI2c {
        fn new(distance_mm: u16) -> Self {
            Self {
                distance_h: (distance_mm >> 8) as u8,
                distance_l: distance_mm as u8,
            }
        }
    }

    impl I2cBus for StubI2c {
        type Error = I2cError;
        fn write(&mut self, _addr: u8, _data: &[u8]) -> Result<(), I2cError> {
            Ok(())
        }
        fn read(&mut self, _addr: u8, buf: &mut [u8]) -> Result<(), I2cError> {
            buf[0] = self.distance_h;
            buf[1] = self.distance_l;
            Ok(())
        }
        fn write_read(&mut self, addr: u8, write: &[u8], buf: &mut [u8]) -> Result<(), I2cError> {
            // モデルID確認リクエスト
            if write[0] == 0xC0 {
                buf[0] = 0xEE; // valid model ID
            } else if write[0] == 0x13 {
                buf[0] = 0x01; // measurement ready (bit0 set)
            } else {
                buf[0] = self.distance_h;
                if buf.len() > 1 {
                    buf[1] = self.distance_l;
                }
            }
            let _ = addr;
            Ok(())
        }
    }

    #[test]
    fn vl53l0x_reads_distance_correctly() {
        let i2c = StubI2c::new(200);
        let mut sensor = Vl53l0xSensor::new(i2c, VL53L0X_ADDRESS).unwrap();
        let reading = sensor.read_distance().unwrap();
        assert_eq!(reading.distance_mm, 200);
    }

    #[test]
    fn vl53l0x_reads_max_range() {
        let i2c = StubI2c::new(2000);
        let mut sensor = Vl53l0xSensor::new(i2c, VL53L0X_ADDRESS).unwrap();
        let reading = sensor.read_distance().unwrap();
        assert_eq!(reading.distance_mm, 2000);
    }

    #[test]
    fn vl53l0x_rejects_wrong_model_id() {
        struct WrongIdI2c;
        impl I2cBus for WrongIdI2c {
            type Error = I2cError;
            fn write(&mut self, _addr: u8, _data: &[u8]) -> Result<(), I2cError> {
                Ok(())
            }
            fn read(&mut self, _addr: u8, _buf: &mut [u8]) -> Result<(), I2cError> {
                Ok(())
            }
            fn write_read(
                &mut self,
                _addr: u8,
                _write: &[u8],
                buf: &mut [u8],
            ) -> Result<(), I2cError> {
                buf[0] = 0xFF; // wrong model ID
                Ok(())
            }
        }
        let result = Vl53l0xSensor::new(WrongIdI2c, VL53L0X_ADDRESS);
        assert!(matches!(result, Err(SensorError::InvalidReading)));
    }

    #[test]
    fn vl53l0x_new_fails_on_bus_error() {
        struct FailI2c;
        impl I2cBus for FailI2c {
            type Error = I2cError;
            fn write(&mut self, _: u8, _: &[u8]) -> Result<(), I2cError> { Ok(()) }
            fn read(&mut self, _: u8, _: &mut [u8]) -> Result<(), I2cError> { Ok(()) }
            fn write_read(&mut self, _: u8, _: &[u8], _: &mut [u8]) -> Result<(), I2cError> {
                Err(I2cError::BusError)
            }
        }
        let result = Vl53l0xSensor::new(FailI2c, VL53L0X_ADDRESS);
        assert!(matches!(result, Err(SensorError::BusError)));
    }

    #[test]
    fn vl53l0x_returns_busy_when_measurement_not_ready() {
        struct NeverReadyI2c;
        impl I2cBus for NeverReadyI2c {
            type Error = I2cError;
            fn write(&mut self, _: u8, _: &[u8]) -> Result<(), I2cError> { Ok(()) }
            fn read(&mut self, _: u8, _: &mut [u8]) -> Result<(), I2cError> { Ok(()) }
            fn write_read(&mut self, _: u8, write: &[u8], buf: &mut [u8]) -> Result<(), I2cError> {
                if write[0] == 0xC0 { buf[0] = 0xEE; } // identity ok
                else { buf[0] = 0x00; } // never ready
                Ok(())
            }
        }
        let mut sensor = Vl53l0xSensor::new(NeverReadyI2c, VL53L0X_ADDRESS).unwrap();
        assert_eq!(sensor.read_distance(), Err(SensorError::Busy));
    }
}
