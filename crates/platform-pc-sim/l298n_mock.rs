//! Host-side L298N dual motor driver mock.
//!
//! - [`MockL298nChannel`] implements [`DriveMotor`] for single-channel unit tests.
//! - [`MockL298nDevice`] wraps two channels and implements [`DualMotorDriver`].
//!
//! Cloned instances of `MockL298nChannel` share the same internal state.
//!
//! # `component_sim::MockDualMotorDriver` との使い分け
//!
//! `component_sim::MockDualMotorDriver` はダッシュボードのアプリケーション層で使う
//! 軽量モックで、左右独立のコマンド履歴や呼び出し回数の記録機能を持ちません。
//! `MockL298nChannel` / `MockL298nDevice` はトレイトの単体テストに特化した豊富な観測 API
//! （`history()`・`call_count()`・クローン間状態共有）を提供します。
//!
//! # Examples
//!
//! ```
//! use hal_api::actuator::{DualMotorDriver, MotorCommand, MotorDirection};
//! use platform_pc_sim::l298n_mock::MockL298nDevice;
//!
//! let mut device = MockL298nDevice::new();
//! device.apply_channels(
//!     MotorCommand::new(MotorDirection::Forward, 42),
//!     MotorCommand::new(MotorDirection::Reverse, 30),
//! ).unwrap();
//! assert_eq!(device.left.current_command().direction, MotorDirection::Forward);
//! assert_eq!(device.right.current_command().direction, MotorDirection::Reverse);
//! ```

use hal_api::actuator::{DriveMotor, DualMotorDriver, MotorCommand, MotorDirection};
use hal_api::error::ActuatorError;
use std::cell::RefCell;
use std::rc::Rc;
use std::vec::Vec;

/// Demo motor command sequence: forward → steer left → steer right → reverse → coast.
pub fn demo_commands() -> Vec<MotorCommand> {
    vec![
        MotorCommand::new(MotorDirection::Forward, 42),
        MotorCommand::new(MotorDirection::Forward, 28),
        MotorCommand::new(MotorDirection::Forward, 46),
        MotorCommand::new(MotorDirection::Reverse, 30),
        MotorCommand::new(MotorDirection::Coast, 0),
    ]
}

#[derive(Debug)]
struct MockL298nState {
    command: MotorCommand,
    history: Vec<MotorCommand>,
    call_count: usize,
}

impl Default for MockL298nState {
    fn default() -> Self {
        Self {
            command: MotorCommand::new(MotorDirection::Coast, 0),
            history: Vec::new(),
            call_count: 0,
        }
    }
}

/// Mock single-channel motor driver.
///
/// Implements [`DriveMotor`] for host-side unit tests.
/// Cloned instances share the same internal state.
#[derive(Clone, Debug, Default)]
pub struct MockL298nChannel {
    state: Rc<RefCell<MockL298nState>>,
}

impl MockL298nChannel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Most recent command applied.
    pub fn current_command(&self) -> MotorCommand {
        self.state.borrow().command
    }

    /// Full history of commands applied since construction.
    pub fn history(&self) -> Vec<MotorCommand> {
        self.state.borrow().history.clone()
    }

    /// Number of times `apply` was called.
    pub fn call_count(&self) -> usize {
        self.state.borrow().call_count
    }
}

impl DriveMotor for MockL298nChannel {
    type Error = ActuatorError;

    fn apply(&mut self, command: MotorCommand) -> Result<(), Self::Error> {
        if command.duty_percent > 100 {
            return Err(ActuatorError::InvalidCommand);
        }
        let mut state = self.state.borrow_mut();
        state.command = command;
        state.history.push(command);
        state.call_count += 1;
        Ok(())
    }
}

/// Mock dual-channel L298N motor driver.
///
/// Implements [`DualMotorDriver`] using two `MockL298nChannel` instances.
/// The `left` and `right` fields are publicly accessible for state inspection.
#[derive(Clone, Debug, Default)]
pub struct MockL298nDevice {
    pub left: MockL298nChannel,
    pub right: MockL298nChannel,
}

impl MockL298nDevice {
    pub fn new() -> Self {
        Self::default()
    }
}

impl DualMotorDriver for MockL298nDevice {
    type Error = ActuatorError;

    fn apply_channels(
        &mut self,
        left: MotorCommand,
        right: MotorCommand,
    ) -> Result<(), Self::Error> {
        // Validate both channels before updating any state (atomic semantics).
        if left.duty_percent > 100 || right.duty_percent > 100 {
            return Err(ActuatorError::InvalidCommand);
        }
        self.left.apply(left)?;
        self.right.apply(right)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_channel_records_command() {
        let mut ch = MockL298nChannel::new();
        let cmd = MotorCommand::new(MotorDirection::Forward, 50);
        ch.apply(cmd).unwrap();
        assert_eq!(ch.current_command().direction, MotorDirection::Forward);
        assert_eq!(ch.current_command().duty_percent, 50);
        assert_eq!(ch.call_count(), 1);
    }

    #[test]
    fn mock_channel_rejects_duty_over_100() {
        let mut ch = MockL298nChannel::new();
        assert_eq!(
            ch.apply(MotorCommand::new(MotorDirection::Forward, 101)),
            Err(ActuatorError::InvalidCommand)
        );
    }

    #[test]
    fn mock_channel_records_history() {
        let mut ch = MockL298nChannel::new();
        for cmd in demo_commands() {
            ch.apply(cmd).unwrap();
        }
        assert_eq!(ch.history(), demo_commands());
    }

    #[test]
    fn mock_channel_clones_share_state() {
        let ch = MockL298nChannel::new();
        let mut ch2 = ch.clone();
        ch2.apply(MotorCommand::new(MotorDirection::Brake, 0))
            .unwrap();
        assert_eq!(ch.current_command().direction, MotorDirection::Brake);
    }

    #[test]
    fn mock_l298n_device_routes_to_both_channels() {
        let mut device = MockL298nDevice::new();
        let left = MotorCommand::new(MotorDirection::Forward, 35);
        let right = MotorCommand::new(MotorDirection::Reverse, 20);
        device.apply_channels(left, right).unwrap();
        assert_eq!(
            device.left.current_command().direction,
            MotorDirection::Forward
        );
        assert_eq!(device.left.current_command().duty_percent, 35);
        assert_eq!(
            device.right.current_command().direction,
            MotorDirection::Reverse
        );
        assert_eq!(device.right.current_command().duty_percent, 20);
    }

    #[test]
    fn mock_l298n_device_default_is_coast() {
        let device = MockL298nDevice::new();
        assert_eq!(
            device.left.current_command().direction,
            MotorDirection::Coast
        );
        assert_eq!(
            device.right.current_command().direction,
            MotorDirection::Coast
        );
    }

    #[test]
    fn demo_commands_are_all_valid() {
        let mut ch = MockL298nChannel::new();
        for cmd in demo_commands() {
            assert!(ch.apply(cmd).is_ok());
        }
    }

    #[test]
    fn mock_channel_accepts_max_valid_duty() {
        let mut ch = MockL298nChannel::new();
        assert!(ch
            .apply(MotorCommand::new(MotorDirection::Forward, 100))
            .is_ok());
        assert_eq!(ch.current_command().duty_percent, 100);
    }

    #[test]
    fn mock_channel_state_unchanged_on_rejection() {
        let mut ch = MockL298nChannel::new();
        ch.apply(MotorCommand::new(MotorDirection::Forward, 50))
            .unwrap();
        let _ = ch.apply(MotorCommand::new(MotorDirection::Forward, 101));
        assert_eq!(ch.current_command().duty_percent, 50);
        assert_eq!(ch.call_count(), 1);
        assert_eq!(ch.history().len(), 1);
    }

    #[test]
    fn mock_l298n_device_apply_channels_is_atomic_on_right_failure() {
        let mut device = MockL298nDevice::new();
        let result = device.apply_channels(
            MotorCommand::new(MotorDirection::Forward, 50),
            MotorCommand::new(MotorDirection::Forward, 101),
        );
        assert_eq!(result, Err(ActuatorError::InvalidCommand));
        assert_eq!(
            device.left.current_command().direction,
            MotorDirection::Coast
        );
        assert_eq!(device.left.call_count(), 0);
    }
}
