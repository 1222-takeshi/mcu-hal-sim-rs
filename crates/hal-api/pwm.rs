//! PWM output abstraction.

/// PWM 出力ピンの抽象。
///
/// duty は 0–100 のパーセント単位。0 で常に LOW、100 で常に HIGH。
///
/// # Examples
///
/// ```
/// use hal_api::pwm::PwmOutput;
/// use hal_api::error::ActuatorError;
///
/// struct FixedPwm;
///
/// impl PwmOutput for FixedPwm {
///     type Error = ActuatorError;
///
///     fn set_duty_percent(&mut self, duty: u8) -> Result<(), Self::Error> {
///         if duty > 100 {
///             return Err(ActuatorError::InvalidCommand);
///         }
///         Ok(())
///     }
///
///     fn duty_percent(&self) -> u8 { 0 }
/// }
///
/// let mut pwm = FixedPwm;
/// assert!(pwm.set_duty_percent(50).is_ok());
/// assert_eq!(pwm.set_duty_percent(101), Err(ActuatorError::InvalidCommand));
/// ```
pub trait PwmOutput {
    /// エラー型
    type Error;

    /// デューティ比を設定する（0–100 %）。
    ///
    /// 100 を超える値が渡された場合、実装は `Err(InvalidCommand)` を返すこと。
    fn set_duty_percent(&mut self, duty: u8) -> Result<(), Self::Error>;

    /// 現在のデューティ比を返す（0–100 %）。
    fn duty_percent(&self) -> u8;
}
