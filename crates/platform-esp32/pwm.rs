//! ESP32 PWM 出力アダプタ
//!
//! `embedded-hal` v1.0 の `SetDutyCycle` を実装したピンを受け取り、
//! `hal_api::pwm::PwmOutput` に橋渡しします。
//!
//! # 使用例（コンパイル確認用）
//!
//! ```ignore
//! // esp-hal の LEDC や McPWM ピンを Esp32PwmOutput でラップし、
//! // ServoDriver や L298nChannel と組み合わせる:
//! //
//! // let servo = ServoDriver::new(Esp32PwmOutput::new(ledc_channel));
//! // let motor_ch = L298nChannel::new(
//! //     Esp32OutputPin::new(gpio_in1),
//! //     Esp32OutputPin::new(gpio_in2),
//! //     Esp32PwmOutput::new(ledc_ena),
//! // );
//! ```

use embedded_hal::pwm::{
    Error as EmbeddedPwmError, ErrorKind as EmbeddedPwmErrorKind, SetDutyCycle,
};
use hal_api::error::ActuatorError;
use hal_api::pwm::PwmOutput;

fn map_pwm_error(error: impl EmbeddedPwmError) -> ActuatorError {
    match error.kind() {
        EmbeddedPwmErrorKind::Other => ActuatorError::HardwareError,
        _ => ActuatorError::HardwareError,
    }
}

/// ESP32 向けの PWM 出力ラッパー。
///
/// `embedded-hal` v1.0 の `SetDutyCycle` を実装したチャンネルを受け取り、
/// `hal_api::pwm::PwmOutput`（デューティ比 0–100 %）に橋渡しします。
pub struct Esp32PwmOutput<P> {
    inner: P,
    current_duty: u8,
}

impl<P> Esp32PwmOutput<P> {
    /// ラップ対象の PWM チャンネルからアダプタを生成します。
    pub fn new(inner: P) -> Self {
        Self {
            inner,
            current_duty: 0,
        }
    }

    /// 内部 PWM チャンネルの参照を取得します。
    pub fn inner(&self) -> &P {
        &self.inner
    }

    /// 内部 PWM チャンネルの可変参照を取得します。
    pub fn inner_mut(&mut self) -> &mut P {
        &mut self.inner
    }

    /// 内部 PWM チャンネルを取り出します。
    pub fn into_inner(self) -> P {
        self.inner
    }
}

impl<P> PwmOutput for Esp32PwmOutput<P>
where
    P: SetDutyCycle,
{
    type Error = ActuatorError;

    fn set_duty_percent(&mut self, duty: u8) -> Result<(), Self::Error> {
        if duty > 100 {
            return Err(ActuatorError::InvalidCommand);
        }
        self.inner
            .set_duty_cycle_percent(duty)
            .map_err(map_pwm_error)?;
        self.current_duty = duty;
        Ok(())
    }

    fn duty_percent(&self) -> u8 {
        self.current_duty
    }
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use core::convert::Infallible;

    use super::*;
    use embedded_hal::pwm::SetDutyCycle;

    struct DummyPwm {
        duty: u16,
        max: u16,
    }

    impl DummyPwm {
        fn new(max: u16) -> Self {
            Self { duty: 0, max }
        }
    }

    impl embedded_hal::pwm::ErrorType for DummyPwm {
        type Error = Infallible;
    }

    impl SetDutyCycle for DummyPwm {
        fn max_duty_cycle(&self) -> u16 {
            self.max
        }

        fn set_duty_cycle(&mut self, duty: u16) -> Result<(), Self::Error> {
            self.duty = duty;
            Ok(())
        }
    }

    #[derive(Debug, Clone, Copy)]
    struct DummyPwmError;

    impl embedded_hal::pwm::Error for DummyPwmError {
        fn kind(&self) -> EmbeddedPwmErrorKind {
            EmbeddedPwmErrorKind::Other
        }
    }

    struct FailingPwm;

    impl embedded_hal::pwm::ErrorType for FailingPwm {
        type Error = DummyPwmError;
    }

    impl SetDutyCycle for FailingPwm {
        fn max_duty_cycle(&self) -> u16 {
            1000
        }

        fn set_duty_cycle(&mut self, _duty: u16) -> Result<(), Self::Error> {
            Err(DummyPwmError)
        }
    }

    #[test]
    fn esp32_pwm_output_delegates_to_inner_channel() {
        let inner = DummyPwm::new(1000);
        let mut pwm = Esp32PwmOutput::new(inner);

        pwm.set_duty_percent(50).unwrap();

        assert_eq!(pwm.duty_percent(), 50);
        // max=1000, percent=50 → duty_cycle_fraction(50,100) → 1000*50/100 = 500
        assert_eq!(pwm.inner().duty, 500);
    }

    #[test]
    fn esp32_pwm_output_rejects_duty_over_100() {
        let inner = DummyPwm::new(1000);
        let mut pwm = Esp32PwmOutput::new(inner);

        assert_eq!(
            pwm.set_duty_percent(101),
            Err(ActuatorError::InvalidCommand)
        );
    }

    #[test]
    fn esp32_pwm_output_maps_hardware_errors() {
        let mut pwm = Esp32PwmOutput::new(FailingPwm);

        assert_eq!(pwm.set_duty_percent(50), Err(ActuatorError::HardwareError));
    }

    #[test]
    fn esp32_pwm_output_into_inner_returns_wrapped_channel() {
        let inner = DummyPwm::new(255);
        let pwm = Esp32PwmOutput::new(inner);
        assert_eq!(pwm.into_inner().max, 255);
    }

    #[test]
    fn esp32_pwm_initial_duty_is_zero() {
        let pwm = Esp32PwmOutput::new(DummyPwm::new(1000));
        assert_eq!(pwm.duty_percent(), 0);
    }
}
