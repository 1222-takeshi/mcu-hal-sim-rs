//! Host-side HC-SR04 mock device.

use hal_api::distance::UltrasonicPulseDevice;
use hal_api::error::SensorError;
use std::cell::RefCell;
use std::rc::Rc;
use std::vec::Vec;

#[derive(Debug, Default)]
struct MockHcSr04State {
    echo_sequence_us: Vec<u32>,
    next_index: usize,
    trigger_count: usize,
    loop_forever: bool,
}

#[derive(Clone, Debug, Default)]
pub struct MockHcSr04Device {
    state: Rc<RefCell<MockHcSr04State>>,
}

impl MockHcSr04Device {
    pub fn new(sequence_us: Vec<u32>) -> Self {
        Self {
            state: Rc::new(RefCell::new(MockHcSr04State {
                echo_sequence_us: sequence_us,
                next_index: 0,
                trigger_count: 0,
                loop_forever: false,
            })),
        }
    }

    pub fn looping(sequence_us: Vec<u32>) -> Self {
        let device = Self::new(sequence_us);
        device.state.borrow_mut().loop_forever = true;
        device
    }

    pub fn trigger_count(&self) -> usize {
        self.state.borrow().trigger_count
    }
}

impl UltrasonicPulseDevice for MockHcSr04Device {
    type Error = SensorError;

    fn trigger_and_measure_echo_us(&mut self) -> Result<u32, Self::Error> {
        let mut state = self.state.borrow_mut();
        let echo_us = *state
            .echo_sequence_us
            .get(state.next_index)
            .ok_or(SensorError::NotInitialized)?;
        state.trigger_count += 1;

        if state.loop_forever {
            state.next_index = (state.next_index + 1) % state.echo_sequence_us.len();
        } else if state.next_index + 1 < state.echo_sequence_us.len() {
            state.next_index += 1;
        }

        Ok(echo_us)
    }
}

pub fn demo_echo_pulses_us() -> Vec<u32> {
    vec![1_050, 1_400, 1_870, 820]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hc_sr04_mock_loops_when_requested() {
        let mut device = MockHcSr04Device::looping(vec![100, 200]);

        assert_eq!(device.trigger_and_measure_echo_us().unwrap(), 100);
        assert_eq!(device.trigger_and_measure_echo_us().unwrap(), 200);
        assert_eq!(device.trigger_and_measure_echo_us().unwrap(), 100);
    }

    #[test]
    fn hc_sr04_mock_reports_missing_sequence() {
        let mut device = MockHcSr04Device::new(Vec::new());

        assert_eq!(
            device.trigger_and_measure_echo_us(),
            Err(SensorError::NotInitialized)
        );
    }
}
