//! L298N-style dual H-bridge motor driver.
//!
//! 1ch あたり 2 本の方向制御 GPIO (IN1/IN2) と
//! 1 本の PWM 速度制御ピン (ENA/ENB) で構成されます。
//!
//! # 真理値表 (1 チャンネル)
//!
//! | IN1 | IN2 | 動作         |
//! |-----|-----|--------------|
//! | H   | L   | 正転 (Forward) |
//! | L   | H   | 逆転 (Reverse) |
//! | H   | H   | ブレーキ (Brake) |
//! | L   | L   | フリー (Coast)  |

use hal_api::actuator::{DriveMotor, DualMotorDriver, MotorCommand, MotorDirection};
use hal_api::error::ActuatorError;
use hal_api::gpio::OutputPin;
use hal_api::pwm::PwmOutput;

/// L298N の 1 チャンネル。
///
/// `D1`/`D2` は方向制御ピン、`P` は速度制御 PWM ピン。
pub struct L298nChannel<D1, D2, P> {
    in1: D1,
    in2: D2,
    enable: P,
    current: MotorCommand,
}

impl<D1, D2, P> L298nChannel<D1, D2, P>
where
    D1: OutputPin,
    D2: OutputPin,
    P: PwmOutput,
    D1::Error: Into<ActuatorError>,
    D2::Error: Into<ActuatorError>,
    P::Error: Into<ActuatorError>,
{
    pub fn new(in1: D1, in2: D2, enable: P) -> Self {
        Self {
            in1,
            in2,
            enable,
            current: MotorCommand::new(MotorDirection::Coast, 0),
        }
    }

    /// 最後に適用された指令を返す。
    pub fn current_command(&self) -> MotorCommand {
        self.current
    }

    /// 内部 IN1 ピンへの参照（テスト・観測用）。
    pub fn in1(&self) -> &D1 {
        &self.in1
    }

    /// 内部 IN2 ピンへの参照（テスト・観測用）。
    pub fn in2(&self) -> &D2 {
        &self.in2
    }

    /// 内部 PWM ピンへの参照（テスト・観測用）。
    pub fn enable(&self) -> &P {
        &self.enable
    }
}

impl<D1, D2, P> DriveMotor for L298nChannel<D1, D2, P>
where
    D1: OutputPin,
    D2: OutputPin,
    P: PwmOutput,
    D1::Error: Into<ActuatorError>,
    D2::Error: Into<ActuatorError>,
    P::Error: Into<ActuatorError>,
{
    type Error = ActuatorError;

    fn apply(&mut self, command: MotorCommand) -> Result<(), Self::Error> {
        if command.duty_percent > 100 {
            return Err(ActuatorError::InvalidCommand);
        }
        let (in1_high, in2_high) = match command.direction {
            MotorDirection::Forward => (true, false),
            MotorDirection::Reverse => (false, true),
            MotorDirection::Brake => (true, true),
            MotorDirection::Coast => (false, false),
        };
        self.in1.set(in1_high).map_err(Into::into)?;
        self.in2.set(in2_high).map_err(Into::into)?;
        self.enable
            .set_duty_percent(command.duty_percent)
            .map_err(Into::into)?;
        self.current = command;
        Ok(())
    }
}

/// L298N の 2 チャンネルまとめ。`A` が左、`B` が右チャンネル。
pub struct L298nDualDriver<A, B> {
    channel_a: A,
    channel_b: B,
}

impl<A, B> L298nDualDriver<A, B>
where
    A: DriveMotor<Error = ActuatorError>,
    B: DriveMotor<Error = ActuatorError>,
{
    pub fn new(channel_a: A, channel_b: B) -> Self {
        Self {
            channel_a,
            channel_b,
        }
    }

    /// チャンネル A（左）への参照。
    pub fn channel_a(&self) -> &A {
        &self.channel_a
    }

    /// チャンネル B（右）への参照。
    pub fn channel_b(&self) -> &B {
        &self.channel_b
    }
}

impl<A, B> DualMotorDriver for L298nDualDriver<A, B>
where
    A: DriveMotor<Error = ActuatorError>,
    B: DriveMotor<Error = ActuatorError>,
{
    type Error = ActuatorError;

    fn apply_channels(
        &mut self,
        left: MotorCommand,
        right: MotorCommand,
    ) -> Result<(), Self::Error> {
        self.channel_a.apply(left)?;
        self.channel_b.apply(right)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate std;

    struct RecordingPin {
        level: bool,
        calls: usize,
    }

    impl RecordingPin {
        fn new() -> Self {
            Self {
                level: false,
                calls: 0,
            }
        }
    }

    impl OutputPin for RecordingPin {
        type Error = ActuatorError;

        fn set_high(&mut self) -> Result<(), Self::Error> {
            self.level = true;
            self.calls += 1;
            Ok(())
        }

        fn set_low(&mut self) -> Result<(), Self::Error> {
            self.level = false;
            self.calls += 1;
            Ok(())
        }
    }

    struct RecordingPwm {
        duty: u8,
    }

    impl RecordingPwm {
        fn new() -> Self {
            Self { duty: 0 }
        }
    }

    impl PwmOutput for RecordingPwm {
        type Error = ActuatorError;

        fn set_duty_percent(&mut self, duty: u8) -> Result<(), Self::Error> {
            if duty > 100 {
                return Err(ActuatorError::InvalidCommand);
            }
            self.duty = duty;
            Ok(())
        }

        fn duty_percent(&self) -> u8 {
            self.duty
        }
    }

    fn make_channel() -> L298nChannel<RecordingPin, RecordingPin, RecordingPwm> {
        L298nChannel::new(
            RecordingPin::new(),
            RecordingPin::new(),
            RecordingPwm::new(),
        )
    }

    #[test]
    fn l298n_channel_forward_sets_pins_correctly() {
        let mut ch = make_channel();
        ch.apply(MotorCommand::new(MotorDirection::Forward, 50))
            .unwrap();
        assert!(ch.in1().level);
        assert!(!ch.in2().level);
        assert_eq!(ch.enable().duty_percent(), 50);
        assert_eq!(ch.current_command().direction, MotorDirection::Forward);
    }

    #[test]
    fn l298n_channel_reverse_sets_pins_correctly() {
        let mut ch = make_channel();
        ch.apply(MotorCommand::new(MotorDirection::Reverse, 40))
            .unwrap();
        assert!(!ch.in1().level);
        assert!(ch.in2().level);
        assert_eq!(ch.enable().duty_percent(), 40);
    }

    #[test]
    fn l298n_channel_brake_sets_both_pins_high() {
        let mut ch = make_channel();
        ch.apply(MotorCommand::new(MotorDirection::Brake, 0))
            .unwrap();
        assert!(ch.in1().level);
        assert!(ch.in2().level);
    }

    #[test]
    fn l298n_channel_coast_sets_both_pins_low() {
        let mut ch = make_channel();
        ch.apply(MotorCommand::new(MotorDirection::Coast, 0))
            .unwrap();
        assert!(!ch.in1().level);
        assert!(!ch.in2().level);
    }

    #[test]
    fn l298n_channel_rejects_duty_over_100() {
        let mut ch = make_channel();
        assert_eq!(
            ch.apply(MotorCommand::new(MotorDirection::Forward, 101)),
            Err(ActuatorError::InvalidCommand)
        );
    }

    #[test]
    fn l298n_dual_driver_apply_channels_routes_to_both() {
        let mut driver = L298nDualDriver::new(make_channel(), make_channel());
        let left = MotorCommand::new(MotorDirection::Forward, 35);
        let right = MotorCommand::new(MotorDirection::Reverse, 20);

        driver.apply_channels(left, right).unwrap();

        assert_eq!(
            driver.channel_a().current_command().direction,
            MotorDirection::Forward
        );
        assert_eq!(driver.channel_a().current_command().duty_percent, 35);
        assert_eq!(
            driver.channel_b().current_command().direction,
            MotorDirection::Reverse
        );
        assert_eq!(driver.channel_b().current_command().duty_percent, 20);
    }
}
