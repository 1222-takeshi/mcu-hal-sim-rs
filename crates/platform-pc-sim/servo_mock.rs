//! Host-side Servo mock.
//!
//! `MockServoDevice` implements [`ServoMotor`] directly for use in host-side unit tests.
//! It records angle history and provides demo sweep sequences.
//!
//! For the sim-to-real dashboard path, the real `ServoDriver<MockPwmOutput>` is used
//! instead, as it exercises the full reference-driver code path.
//!
//! # `component_sim::MockServoMotor` との使い分け
//!
//! `component_sim::MockServoMotor` はダッシュボードのアプリケーション層で使う
//! 軽量モックで、角度履歴や呼び出し回数の記録機能を持ちません。
//! `MockServoDevice` はトレイトの単体テストに特化した豊富な観測 API
//! （`history()`・`call_count()`・クローン間状態共有）を提供します。
//!
//! # 注意: 初期角度
//!
//! `MockServoDevice::new()` の初期角度は `0°` です。
//! 実ハードウェアの `ServoDriver::new()` が `90°`（中立位置）で初期化するのとは異なります。
//! `ServoMotor` トレイトは初期状態を規定しないため、最初の `set_angle_degrees` 呼び出し前に
//! 現在角度を参照するテストは `MockServoDevice` と実ドライバで異なる値を返します。
//!
//! # Examples
//!
//! ```
//! use hal_api::actuator::ServoMotor;
//! use platform_pc_sim::servo_mock::{MockServoDevice, demo_angles};
//!
//! let mut servo = MockServoDevice::new();
//! for angle in demo_angles() {
//!     servo.set_angle_degrees(angle).unwrap();
//! }
//! assert_eq!(servo.history(), demo_angles());
//! ```

use hal_api::actuator::ServoMotor;
use hal_api::error::ActuatorError;
use std::cell::RefCell;
use std::rc::Rc;
use std::vec::Vec;

/// Demo sweep angles: 0° → 45° → 90° → 135° → 180° → 135° → 90° → 45°.
pub fn demo_angles() -> Vec<u16> {
    vec![0, 45, 90, 135, 180, 135, 90, 45]
}

#[derive(Debug, Default)]
struct MockServoState {
    angle_degrees: u16,
    history: Vec<u16>,
    call_count: usize,
}

/// Mock servo that records angle commands.
///
/// Implements [`ServoMotor`] for host-side unit tests.
/// Cloned instances share the same internal state.
#[derive(Clone, Debug, Default)]
pub struct MockServoDevice {
    state: Rc<RefCell<MockServoState>>,
}

impl MockServoDevice {
    pub fn new() -> Self {
        Self::default()
    }

    /// Current angle set by the last `set_angle_degrees` call.
    pub fn current_angle(&self) -> u16 {
        self.state.borrow().angle_degrees
    }

    /// Full history of angles set since construction.
    pub fn history(&self) -> Vec<u16> {
        self.state.borrow().history.clone()
    }

    /// Number of times `set_angle_degrees` was called.
    pub fn call_count(&self) -> usize {
        self.state.borrow().call_count
    }
}

impl ServoMotor for MockServoDevice {
    type Error = ActuatorError;

    fn set_angle_degrees(&mut self, angle_degrees: u16) -> Result<(), Self::Error> {
        if angle_degrees > 180 {
            return Err(ActuatorError::InvalidCommand);
        }
        let mut state = self.state.borrow_mut();
        state.angle_degrees = angle_degrees;
        state.history.push(angle_degrees);
        state.call_count += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_servo_records_angle() {
        let mut servo = MockServoDevice::new();
        servo.set_angle_degrees(90).unwrap();
        assert_eq!(servo.current_angle(), 90);
        assert_eq!(servo.call_count(), 1);
    }

    #[test]
    fn mock_servo_rejects_angle_over_180() {
        let mut servo = MockServoDevice::new();
        assert_eq!(
            servo.set_angle_degrees(181),
            Err(ActuatorError::InvalidCommand)
        );
    }

    #[test]
    fn mock_servo_records_history() {
        let mut servo = MockServoDevice::new();
        for angle in demo_angles() {
            servo.set_angle_degrees(angle).unwrap();
        }
        assert_eq!(servo.history(), demo_angles());
    }

    #[test]
    fn mock_servo_clones_share_state() {
        let servo = MockServoDevice::new();
        let mut servo2 = servo.clone();
        servo2.set_angle_degrees(45).unwrap();
        assert_eq!(servo.current_angle(), 45);
    }

    #[test]
    fn mock_servo_initial_angle_is_zero() {
        let servo = MockServoDevice::new();
        assert_eq!(servo.current_angle(), 0);
        assert!(servo.history().is_empty());
    }

    #[test]
    fn demo_angles_are_valid_servo_range() {
        let mut servo = MockServoDevice::new();
        for angle in demo_angles() {
            assert!(servo.set_angle_degrees(angle).is_ok());
        }
    }

    #[test]
    fn mock_servo_accepts_boundary_angles() {
        let mut servo = MockServoDevice::new();
        assert!(servo.set_angle_degrees(0).is_ok());
        assert!(servo.set_angle_degrees(180).is_ok());
        assert_eq!(servo.current_angle(), 180);
        assert_eq!(servo.call_count(), 2);
    }

    #[test]
    fn mock_servo_state_unchanged_on_rejection() {
        let mut servo = MockServoDevice::new();
        servo.set_angle_degrees(90).unwrap();
        let _ = servo.set_angle_degrees(200);
        assert_eq!(servo.current_angle(), 90);
        assert_eq!(servo.call_count(), 1);
        assert_eq!(servo.history(), vec![90]);
    }
}
