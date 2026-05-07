//! ESP32 アダプタと reference-drivers を合成した便利型エイリアス群。
//!
//! ジェネリクスをすべて明示する代わりにこれらのエイリアスを使うと、
//! 典型的な ESP32 + センサー/アクチュエータのスタックを簡潔に記述できます。
//!
//! # 使用例
//!
//! ```ignore
//! // ESP32 立ち上げ（esp-hal の型をジェネリックアダプタに接続）:
//! //
//! // let i2c_raw = esp_hal::i2c::master::I2c::new(peripherals.I2C0, config);
//! // let mut i2c = Esp32I2c::new(i2c_raw);
//! //
//! // 型エイリアスを使った初期化:
//! // let mut bme = Esp32Bme280::new(&mut i2c, ...);
//! // let mut servo = Esp32ServoDriver::new(Esp32PwmOutput::new(ledc_ch));
//! ```

use crate::dht22::Esp32Dht22Sensor;
use crate::gpio::Esp32OutputPin;
use crate::i2c::Esp32I2c;
use crate::l298n::{L298nChannel, L298nDualDriver};
use crate::pwm::Esp32PwmOutput;
use crate::servo::ServoDriver;
use reference_drivers::bh1750::Bh1750Sensor;
use reference_drivers::bme280::Bme280Sensor;
use reference_drivers::ds3231::Ds3231Sensor;
use reference_drivers::lcd1602::Lcd1602Display;
use reference_drivers::mpu6050::Mpu6050Sensor;
use reference_drivers::sgp30::Sgp30Sensor;
use reference_drivers::ssd1306::Ssd1306Display;
use reference_drivers::vl53l0x::Vl53l0xSensor;

// ---------------------------------------------------------------------------
// センサー型エイリアス
// ---------------------------------------------------------------------------

/// ESP32 I2C バスに接続した BME280 温湿度・気圧センサー。
pub type Esp32Bme280<I> = Bme280Sensor<Esp32I2c<I>>;

/// ESP32 I2C バスに接続した LCD1602 キャラクタディスプレイ。
///
/// `D` は `embedded_hal::delay::DelayNs` 実装（[`crate::delay::Esp32Delay`] を推奨）。
pub type Esp32Lcd1602<I, D> = Lcd1602Display<Esp32I2c<I>, D>;

/// ESP32 I2C バスに接続した MPU6050 IMU センサー。
pub type Esp32Mpu6050<I> = Mpu6050Sensor<Esp32I2c<I>>;

/// ESP32 I2C バスに接続した BH1750 照度センサー。
pub type Esp32Bh1750<I> = Bh1750Sensor<Esp32I2c<I>>;

/// ESP32 I2C バスに接続した DS3231 RTC。
pub type Esp32Ds3231<I> = Ds3231Sensor<Esp32I2c<I>>;

/// ESP32 I2C バスに接続した SGP30 CO₂/VOC センサー。
pub type Esp32Sgp30<I> = Sgp30Sensor<Esp32I2c<I>>;

/// ESP32 I2C バスに接続した VL53L0X ToF 距離センサー。
pub type Esp32Vl53l0x<I> = Vl53l0xSensor<Esp32I2c<I>>;

/// ESP32 I2C バスに接続した SSD1306 OLED ディスプレイ。
pub type Esp32Ssd1306<I> = Ssd1306Display<Esp32I2c<I>>;

/// ESP32 GPIO を使った DHT22 温湿度センサー。
///
/// `P` はオープンドレイン設定の GPIO ピン（`InputPin + OutputPin` 兼用）。
/// `D` はマイクロ秒精度の delay 実装（`embedded_hal::delay::DelayNs`）。
/// 実装は現在スタブです（[`crate::dht22::Esp32Dht22RawDevice`] 参照）。
pub type Esp32Dht22<P, D> = Esp32Dht22Sensor<P, D>;

// ---------------------------------------------------------------------------
// アクチュエータ型エイリアス
// ---------------------------------------------------------------------------

/// ESP32 PWM チャンネルを使ったサーボモータドライバ。
pub type Esp32ServoDriver<P> = ServoDriver<Esp32PwmOutput<P>>;

/// ESP32 GPIO (IN1/IN2) + PWM (ENA) で構成した L298N 1 チャンネル。
pub type Esp32L298nChannel<IN1, IN2, ENA> =
    L298nChannel<Esp32OutputPin<IN1>, Esp32OutputPin<IN2>, Esp32PwmOutput<ENA>>;

/// 2 チャンネル L298N デュアルモータドライバ（各チャンネルを ESP32 GPIO + PWM に接続）。
pub type Esp32L298nDualDriver<IN1A, IN2A, ENAA, IN1B, IN2B, ENAB> =
    L298nDualDriver<Esp32L298nChannel<IN1A, IN2A, ENAA>, Esp32L298nChannel<IN1B, IN2B, ENAB>>;

/// 両チャンネルで同一のピン型を使う典型的なケース向けの簡略エイリアス。
///
/// `IN` は IN1/IN2 の GPIO ピン型（4 本すべて共通）、
/// `ENA` は ENA の PWM チャンネル型（両チャンネル共通）。
pub type Esp32L298nDualDriverSimple<IN, ENA> = Esp32L298nDualDriver<IN, IN, ENA, IN, IN, ENA>;

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    //! 型エイリアスが正しく解決され、エンドツーエンドで動作することを確認するスモークテスト。

    use core::convert::Infallible;

    use embedded_hal::digital::{ErrorType as DigitalErrorType, OutputPin as EmbeddedOutputPin};
    use embedded_hal::pwm::{ErrorType as PwmErrorType, SetDutyCycle};
    use hal_api::actuator::{
        DriveMotor, DualMotorDriver, MotorCommand, MotorDirection, ServoMotor,
    };

    use super::*;

    // ── Dummy embedded-hal stubs ──────────────────────────────────────────

    struct DummyOutputPin;
    impl DigitalErrorType for DummyOutputPin {
        type Error = Infallible;
    }
    impl EmbeddedOutputPin for DummyOutputPin {
        fn set_high(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
        fn set_low(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    struct DummyPwm;
    impl PwmErrorType for DummyPwm {
        type Error = Infallible;
    }
    impl SetDutyCycle for DummyPwm {
        fn max_duty_cycle(&self) -> u16 {
            1000
        }
        fn set_duty_cycle(&mut self, _duty: u16) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    // ── Tests ─────────────────────────────────────────────────────────────

    #[test]
    fn esp32_servo_driver_type_alias_resolves_and_operates() {
        let mut servo: Esp32ServoDriver<DummyPwm> =
            Esp32ServoDriver::new(Esp32PwmOutput::new(DummyPwm));

        servo.set_angle_degrees(0).unwrap();
        assert_eq!(servo.current_angle(), 0);

        servo.set_angle_degrees(90).unwrap();
        assert_eq!(servo.current_angle(), 90);

        servo.set_angle_degrees(180).unwrap();
        assert_eq!(servo.current_angle(), 180);
    }

    #[test]
    fn esp32_servo_driver_type_alias_rejects_angle_beyond_180() {
        let mut servo: Esp32ServoDriver<DummyPwm> =
            Esp32ServoDriver::new(Esp32PwmOutput::new(DummyPwm));
        assert!(servo.set_angle_degrees(181).is_err());
    }

    #[test]
    fn esp32_l298n_channel_type_alias_resolves_and_drives_forward() {
        let mut ch: Esp32L298nChannel<DummyOutputPin, DummyOutputPin, DummyPwm> =
            Esp32L298nChannel::new(
                Esp32OutputPin::new(DummyOutputPin),
                Esp32OutputPin::new(DummyOutputPin),
                Esp32PwmOutput::new(DummyPwm),
            );

        ch.apply(MotorCommand::new(MotorDirection::Forward, 75))
            .unwrap();
        assert_eq!(ch.current_command().duty_percent, 75);
        assert_eq!(ch.current_command().direction, MotorDirection::Forward);
    }

    #[test]
    fn esp32_l298n_channel_type_alias_rejects_duty_over_100() {
        let mut ch: Esp32L298nChannel<DummyOutputPin, DummyOutputPin, DummyPwm> =
            Esp32L298nChannel::new(
                Esp32OutputPin::new(DummyOutputPin),
                Esp32OutputPin::new(DummyOutputPin),
                Esp32PwmOutput::new(DummyPwm),
            );
        assert!(ch
            .apply(MotorCommand::new(MotorDirection::Forward, 101))
            .is_err());
    }

    #[test]
    fn esp32_l298n_dual_driver_type_alias_resolves_and_applies_commands() {
        type Driver = Esp32L298nDualDriver<
            DummyOutputPin,
            DummyOutputPin,
            DummyPwm,
            DummyOutputPin,
            DummyOutputPin,
            DummyPwm,
        >;

        let make_ch = || {
            Esp32L298nChannel::new(
                Esp32OutputPin::new(DummyOutputPin),
                Esp32OutputPin::new(DummyOutputPin),
                Esp32PwmOutput::new(DummyPwm),
            )
        };

        let mut driver = Driver::new(make_ch(), make_ch());
        driver
            .apply_channels(
                MotorCommand::new(MotorDirection::Forward, 50),
                MotorCommand::new(MotorDirection::Reverse, 30),
            )
            .unwrap();

        assert_eq!(
            driver.channel_a().current_command().direction,
            MotorDirection::Forward
        );
        assert_eq!(
            driver.channel_b().current_command().direction,
            MotorDirection::Reverse
        );
    }

    #[test]
    fn esp32_l298n_dual_driver_simple_type_alias_resolves() {
        type Driver = Esp32L298nDualDriverSimple<DummyOutputPin, DummyPwm>;

        let make_ch = || {
            Esp32L298nChannel::new(
                Esp32OutputPin::new(DummyOutputPin),
                Esp32OutputPin::new(DummyOutputPin),
                Esp32PwmOutput::new(DummyPwm),
            )
        };

        let mut driver = Driver::new(make_ch(), make_ch());
        driver
            .apply_channels(
                MotorCommand::new(MotorDirection::Forward, 60),
                MotorCommand::new(MotorDirection::Brake, 0),
            )
            .unwrap();

        assert_eq!(driver.channel_a().current_command().duty_percent, 60);
    }
}
