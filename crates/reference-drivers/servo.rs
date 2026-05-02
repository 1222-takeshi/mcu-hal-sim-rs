//! Servo motor driver.
//!
//! RC サーボの標準的なパルス幅（1–2ms / 20ms 周期）を
//! PWM デューティ比 5–10 % にマッピングして角度制御を行います。

use hal_api::actuator::ServoMotor;
use hal_api::error::ActuatorError;
use hal_api::pwm::PwmOutput;

/// 50 Hz 信号の 0° に対応するデューティ比（1ms / 20ms = 5 %）
const SERVO_DUTY_MIN: u8 = 5;
/// 50 Hz 信号の 180° に対応するデューティ比（2ms / 20ms = 10 %）
const SERVO_DUTY_MAX: u8 = 10;

/// PWM ピンを使ったサーボドライバ。
///
/// 0–180 度の角度指令を PWM デューティ比 5–10 % に線形変換します。
pub struct ServoDriver<P> {
    pwm: P,
    current_angle: u16,
}

impl<P> ServoDriver<P>
where
    P: PwmOutput,
    P::Error: Into<ActuatorError>,
{
    /// 90 度（中立位置）で初期化する。
    pub fn new(pwm: P) -> Self {
        Self {
            pwm,
            current_angle: 90,
        }
    }

    /// 最後に設定した角度を返す。
    pub fn current_angle(&self) -> u16 {
        self.current_angle
    }

    /// 内部 PWM への参照（テスト・観測用）。
    pub fn pwm(&self) -> &P {
        &self.pwm
    }
}

impl<P> ServoMotor for ServoDriver<P>
where
    P: PwmOutput,
    P::Error: Into<ActuatorError>,
{
    type Error = ActuatorError;

    fn set_angle_degrees(&mut self, angle_degrees: u16) -> Result<(), Self::Error> {
        if angle_degrees > 180 {
            return Err(ActuatorError::InvalidCommand);
        }
        let duty = SERVO_DUTY_MIN
            + (((angle_degrees as u32) * (SERVO_DUTY_MAX - SERVO_DUTY_MIN) as u32) / 180) as u8;
        self.pwm.set_duty_percent(duty).map_err(Into::into)?;
        self.current_angle = angle_degrees;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate std;

    struct SpyPwm {
        duty: u8,
        calls: usize,
    }

    impl SpyPwm {
        fn new() -> Self {
            Self { duty: 0, calls: 0 }
        }
    }

    impl PwmOutput for SpyPwm {
        type Error = ActuatorError;

        fn set_duty_percent(&mut self, duty: u8) -> Result<(), Self::Error> {
            if duty > 100 {
                return Err(ActuatorError::InvalidCommand);
            }
            self.duty = duty;
            self.calls += 1;
            Ok(())
        }

        fn duty_percent(&self) -> u8 {
            self.duty
        }
    }

    #[test]
    fn servo_driver_maps_zero_degrees_to_min_duty() {
        let mut driver = ServoDriver::new(SpyPwm::new());
        driver.set_angle_degrees(0).unwrap();
        assert_eq!(driver.pwm().duty_percent(), SERVO_DUTY_MIN);
        assert_eq!(driver.current_angle(), 0);
    }

    #[test]
    fn servo_driver_maps_180_degrees_to_max_duty() {
        let mut driver = ServoDriver::new(SpyPwm::new());
        driver.set_angle_degrees(180).unwrap();
        assert_eq!(driver.pwm().duty_percent(), SERVO_DUTY_MAX);
        assert_eq!(driver.current_angle(), 180);
    }

    #[test]
    fn servo_driver_rejects_angle_beyond_180() {
        let mut driver = ServoDriver::new(SpyPwm::new());
        assert_eq!(
            driver.set_angle_degrees(181),
            Err(ActuatorError::InvalidCommand)
        );
    }

    #[test]
    fn servo_driver_initial_angle_is_90() {
        let driver = ServoDriver::new(SpyPwm::new());
        assert_eq!(driver.current_angle(), 90);
    }
}
