//! Host-side Servo mock.
//!
//! `MockServoDevice` implements [`ServoMotor`] directly for use in host-side unit tests.
//! It records angle history and provides demo sweep sequences.
//!
//! For the sim-to-real dashboard path, the real `ServoDriver<MockPwmOutput>` is used
//! instead, as it exercises the full reference-driver code path.
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
}
