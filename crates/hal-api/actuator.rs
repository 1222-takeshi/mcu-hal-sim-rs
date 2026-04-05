//! Actuator abstractions for servo and motor control.

/// DC motor の回転方向。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotorDirection {
    Forward,
    Reverse,
    Brake,
    Coast,
}

/// DC motor へ適用する指令値。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MotorCommand {
    pub direction: MotorDirection,
    pub duty_percent: u8,
}

impl MotorCommand {
    pub const fn new(direction: MotorDirection, duty_percent: u8) -> Self {
        Self {
            direction,
            duty_percent,
        }
    }
}

/// サーボモータの抽象。
pub trait ServoMotor {
    type Error;

    /// 角度をデグリーで指定する。有効範囲は 0〜180 度。
    /// 範囲外の値が渡された場合、実装は `Err` を返すこと。
    fn set_angle_degrees(&mut self, angle_degrees: u16) -> Result<(), Self::Error>;
}

/// 1ch の DC motor 出力の抽象。
pub trait DriveMotor {
    type Error;

    fn apply(&mut self, command: MotorCommand) -> Result<(), Self::Error>;
}

/// 2ch モータドライバの抽象。
pub trait DualMotorDriver {
    type Error;

    fn apply_channels(
        &mut self,
        left: MotorCommand,
        right: MotorCommand,
    ) -> Result<(), Self::Error>;
}
