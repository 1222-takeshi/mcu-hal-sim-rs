//! ESP32 向け DHT22 センサアダプタ。
//!
//! DHT22 は単線双方向プロトコル（single-wire）のため、正確な bit 読み取りには
//! マイクロ秒精度の GPIO timing が必要です。このモジュールでは:
//!
//! - `reference_drivers::dht22::Dht22RawDevice` トレイトと `Dht22Sensor<DEV>` を re-export
//! - `Esp32Dht22RawDevice<P, D>` を提供（`embedded-hal` v1.0 対応 InputPin+OutputPin と
//!   `DelayNs` から 40 ビット raw データを読み取るスケルトン実装）
//!
//! # 注意
//!
//! `bit_read_impl` は現在スタブです。実際の esp-idf 実装では
//! `esp_idf_svc::hal::gpio::PinDriver` の開放コレクタ設定と
//! `esp_idf_svc::hal::delay::FreeRtos` を組み合わせて使用してください。

pub use reference_drivers::dht22::{Dht22RawDevice, Dht22Sensor};

use embedded_hal::{
    delay::DelayNs,
    digital::{InputPin, OutputPin},
};
use hal_api::error::SensorError;

/// ESP32 向け DHT22 raw ビット読み取り実装。
///
/// `P` には開放コレクタ設定の GPIO ピン（入出力兼用）を想定しています。
/// 実 HAL では `PinDriver` をそのまま渡せます。
///
/// # 型パラメータ
///
/// * `P` — 入出力兼用ピン（`embedded-hal` v1.0 `InputPin + OutputPin`）
/// * `D` — マイクロ秒精度の delay 実装（`embedded-hal` v1.0 `DelayNs`）
pub struct Esp32Dht22RawDevice<P, D> {
    pin: P,
    delay: D,
}

impl<P, D> Esp32Dht22RawDevice<P, D>
where
    P: InputPin + OutputPin,
    D: DelayNs,
{
    /// ピンと delay から `Esp32Dht22RawDevice` を生成します。
    pub fn new(pin: P, delay: D) -> Self {
        Self { pin, delay }
    }

    /// 内部のピン参照を取得します。
    pub fn pin(&self) -> &P {
        &self.pin
    }

    /// 内部の delay 参照を取得します。
    pub fn delay(&self) -> &D {
        &self.delay
    }
}

impl<P, D> Dht22RawDevice for Esp32Dht22RawDevice<P, D>
where
    P: InputPin + OutputPin,
    D: DelayNs,
{
    type Error = SensorError;

    /// DHT22 から 40 ビット（5 バイト）の raw データを読み取ります。
    ///
    /// # 実装状況
    ///
    /// 現在はスタブです。実際の esp-idf 環境向け実装では:
    ///
    /// 1. ホスト開始信号: LOW 18ms → HIGH 20-40µs
    /// 2. センサ応答待機: HIGH → LOW 80µs → HIGH 80µs
    /// 3. 40 ビット読み取り: 各ビットは LOW 50µs + HIGH (26-28µs=0 / 70µs=1)
    fn read_raw_bytes(&mut self) -> Result<[u8; 5], SensorError> {
        // ホスト開始信号
        self.pin.set_low().map_err(|_| SensorError::BusError)?;
        self.delay.delay_ms(18);
        self.pin.set_high().map_err(|_| SensorError::BusError)?;
        self.delay.delay_us(40);

        // TODO: センサ応答確認と 40 ビット読み取り
        // 実 HAL では下記のループで bit timing を計測する:
        //   for bit_pos in 0..40 { ... }
        // 現段階ではスタブとして NotInitialized を返す。
        // esp-idf 環境で実装する際にこのブロックを置き換えてください。
        Err(SensorError::NotInitialized)
    }
}

/// `Esp32Dht22RawDevice<P, D>` を `Dht22Sensor` でラップした完全型エイリアス。
pub type Esp32Dht22Sensor<P, D> = Dht22Sensor<Esp32Dht22RawDevice<P, D>>;

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_hal::digital::{Error as DigitalError, ErrorKind, ErrorType};

    // ---- テスト用スタブ ----

    #[derive(Debug)]
    struct StubError;

    impl DigitalError for StubError {
        fn kind(&self) -> ErrorKind {
            ErrorKind::Other
        }
    }

    struct StubPin {
        is_high: bool,
    }

    impl ErrorType for StubPin {
        type Error = StubError;
    }

    impl InputPin for StubPin {
        fn is_high(&mut self) -> Result<bool, StubError> {
            Ok(self.is_high)
        }

        fn is_low(&mut self) -> Result<bool, StubError> {
            Ok(!self.is_high)
        }
    }

    impl OutputPin for StubPin {
        fn set_high(&mut self) -> Result<(), StubError> {
            self.is_high = true;
            Ok(())
        }

        fn set_low(&mut self) -> Result<(), StubError> {
            self.is_high = false;
            Ok(())
        }
    }

    struct NoopDelay;

    impl embedded_hal::delay::DelayNs for NoopDelay {
        fn delay_ns(&mut self, _ns: u32) {}
    }

    // ---- テスト ----

    #[test]
    fn esp32_dht22_raw_returns_not_initialized_stub() {
        let pin = StubPin { is_high: true };
        let mut dev = Esp32Dht22RawDevice::new(pin, NoopDelay);
        let result = dev.read_raw_bytes();
        // スタブは NotInitialized を返すことを確認
        assert!(matches!(result, Err(SensorError::NotInitialized)));
    }

    #[test]
    fn esp32_dht22_sensor_wraps_raw_device() {
        let pin = StubPin { is_high: true };
        let dev = Esp32Dht22RawDevice::new(pin, NoopDelay);
        let mut sensor = Dht22Sensor::new(dev);
        // Dht22Sensor::read() がエラーを正しくラップすることを確認
        let result = hal_api::sensor::EnvSensor::read(&mut sensor);
        assert!(result.is_err());
    }
}
