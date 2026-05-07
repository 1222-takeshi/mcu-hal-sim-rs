//! Convenience type aliases for composing ESP32 adapters with reference drivers.
//!
//! Instead of spelling out the full generic parameter chain, use these aliases to
//! construct the typical ESP32 + sensor/actuator stacks.
//!
//! # Usage
//!
//! ```ignore
//! // Typical ESP32 bringup (esp-hal types plug into the generic adapters):
//! //
//! // let i2c_raw = esp_hal::i2c::master::I2c::new(peripherals.I2C0, config);
//! // let mut i2c = Esp32I2c::new(i2c_raw);
//! //
//! // Then use type aliases:
//! // let mut bme: Esp32Bme280<_> = Bme280Sensor::new(&mut i2c, ...);
//! // let mut servo = Esp32ServoDriver::new(Esp32PwmOutput::new(ledc_ch));
//! ```

use crate::gpio::Esp32OutputPin;
use crate::i2c::Esp32I2c;
use crate::pwm::Esp32PwmOutput;
use reference_drivers::bh1750::Bh1750Sensor;
use reference_drivers::bme280::Bme280Sensor;
use reference_drivers::ds3231::Ds3231Sensor;
use reference_drivers::l298n::{L298nChannel, L298nDualDriver};
use reference_drivers::lcd1602::Lcd1602Display;
use reference_drivers::mpu6050::Mpu6050Sensor;
use reference_drivers::servo::ServoDriver;
use reference_drivers::sgp30::Sgp30Sensor;
use reference_drivers::ssd1306::Ssd1306Display;
use reference_drivers::vl53l0x::Vl53l0xSensor;

// ---------------------------------------------------------------------------
// Sensor type aliases
// ---------------------------------------------------------------------------

/// BME280 temperature/humidity/pressure sensor on an ESP32 I2C bus.
pub type Esp32Bme280<I> = Bme280Sensor<Esp32I2c<I>>;

/// LCD1602 character display on an ESP32 I2C bus.
///
/// `D` must implement `embedded_hal::delay::DelayNs` — use [`crate::delay::Esp32Delay`].
pub type Esp32Lcd1602<I, D> = Lcd1602Display<Esp32I2c<I>, D>;

/// MPU6050 IMU sensor on an ESP32 I2C bus.
pub type Esp32Mpu6050<I> = Mpu6050Sensor<Esp32I2c<I>>;

/// BH1750 ambient light sensor on an ESP32 I2C bus.
pub type Esp32Bh1750<I> = Bh1750Sensor<Esp32I2c<I>>;

/// DS3231 RTC on an ESP32 I2C bus.
pub type Esp32Ds3231<I> = Ds3231Sensor<Esp32I2c<I>>;

/// SGP30 gas/VOC sensor on an ESP32 I2C bus.
pub type Esp32Sgp30<I> = Sgp30Sensor<Esp32I2c<I>>;

/// VL53L0X time-of-flight distance sensor on an ESP32 I2C bus.
pub type Esp32Vl53l0x<I> = Vl53l0xSensor<Esp32I2c<I>>;

/// SSD1306 OLED display on an ESP32 I2C bus.
pub type Esp32Ssd1306<I> = Ssd1306Display<Esp32I2c<I>>;

// ---------------------------------------------------------------------------
// Actuator type aliases
// ---------------------------------------------------------------------------

/// Servo motor driver using an ESP32 PWM output channel.
pub type Esp32ServoDriver<P> = ServoDriver<Esp32PwmOutput<P>>;

/// One L298N motor channel wired to ESP32 GPIO (IN1, IN2) and PWM (ENA).
pub type Esp32L298nChannel<IN1, IN2, ENA> =
    L298nChannel<Esp32OutputPin<IN1>, Esp32OutputPin<IN2>, Esp32PwmOutput<ENA>>;

/// Dual L298N motor driver with two channels, each wired to ESP32 GPIO and PWM.
pub type Esp32L298nDualDriver<IN1A, IN2A, ENAA, IN1B, IN2B, ENAB> =
    L298nDualDriver<Esp32L298nChannel<IN1A, IN2A, ENAA>, Esp32L298nChannel<IN1B, IN2B, ENAB>>;

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    //! Smoke tests: verify that the type aliases resolve and compose correctly
    //! using the same dummy stubs as the other bridge tests.

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
}
