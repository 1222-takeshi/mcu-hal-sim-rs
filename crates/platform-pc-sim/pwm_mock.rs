//! Host-side PWM output mock.

use hal_api::error::ActuatorError;
use hal_api::pwm::PwmOutput;
use std::cell::RefCell;
use std::rc::Rc;
use std::vec::Vec;

#[derive(Debug, Default)]
struct MockPwmState {
    duty_percent: u8,
    history: Vec<u8>,
}

/// PWM 出力のモック実装。
///
/// デューティ比の変更履歴を記録します。
/// クローン間で内部状態を共有するため、観測用インスタンスからも確認できます。
#[derive(Clone, Debug, Default)]
pub struct MockPwmOutput {
    state: Rc<RefCell<MockPwmState>>,
}

impl MockPwmOutput {
    pub fn new() -> Self {
        Self::default()
    }

    /// 現在のデューティ比を返す。
    pub fn current_duty(&self) -> u8 {
        self.state.borrow().duty_percent
    }

    /// 設定されたデューティ比の履歴を返す。
    pub fn history(&self) -> Vec<u8> {
        self.state.borrow().history.clone()
    }

    /// 設定回数を返す。
    pub fn call_count(&self) -> usize {
        self.state.borrow().history.len()
    }
}

impl PwmOutput for MockPwmOutput {
    type Error = ActuatorError;

    fn set_duty_percent(&mut self, duty: u8) -> Result<(), Self::Error> {
        if duty > 100 {
            return Err(ActuatorError::InvalidCommand);
        }
        let mut state = self.state.borrow_mut();
        state.duty_percent = duty;
        state.history.push(duty);
        Ok(())
    }

    fn duty_percent(&self) -> u8 {
        self.state.borrow().duty_percent
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hal_api::pwm::PwmOutput;

    #[test]
    fn mock_pwm_tracks_duty_history() {
        let mut pwm = MockPwmOutput::new();
        pwm.set_duty_percent(25).unwrap();
        pwm.set_duty_percent(50).unwrap();

        assert_eq!(pwm.current_duty(), 50);
        assert_eq!(pwm.history(), vec![25, 50]);
        assert_eq!(pwm.call_count(), 2);
    }

    #[test]
    fn mock_pwm_rejects_duty_over_100() {
        let mut pwm = MockPwmOutput::new();
        assert_eq!(
            pwm.set_duty_percent(101),
            Err(ActuatorError::InvalidCommand)
        );
    }

    #[test]
    fn mock_pwm_clone_shares_state() {
        let mut pwm = MockPwmOutput::new();
        let observer = pwm.clone();

        pwm.set_duty_percent(75).unwrap();

        assert_eq!(observer.current_duty(), 75);
        assert_eq!(observer.history(), vec![75]);
    }

    #[test]
    fn mock_pwm_implements_pwm_output_trait() {
        fn accepts_pwm<T: PwmOutput>(p: &mut T) -> bool {
            p.set_duty_percent(50).is_ok()
        }
        let mut pwm = MockPwmOutput::new();
        assert!(accepts_pwm(&mut pwm));
    }
}
