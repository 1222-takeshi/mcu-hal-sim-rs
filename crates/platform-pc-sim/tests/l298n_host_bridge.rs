//! Host bridge test: `L298nDualDriver<L298nChannel<MockPin, MockPin, MockPwmOutput>>` —
//! sim-to-real path verification.
//!
//! Verifies that the real `L298nChannel` and `L298nDualDriver` (from `platform-esp32::l298n`,
//! re-exported from `reference-drivers`) correctly set GPIO direction pins and PWM duty
//! when backed by `MockPin` and `MockPwmOutput` instead of real hardware peripherals.

use hal_api::actuator::{DriveMotor, DualMotorDriver, MotorCommand, MotorDirection};
use hal_api::error::ActuatorError;
use platform_esp32::l298n::{L298nChannel, L298nDualDriver};
use platform_pc_sim::mock_hal::MockPin;
use platform_pc_sim::pwm_mock::MockPwmOutput;

fn make_channel() -> (
    MockPin,
    MockPin,
    MockPwmOutput,
    L298nChannel<MockPin, MockPin, MockPwmOutput>,
) {
    let in1 = MockPin::new(1);
    let in2 = MockPin::new(2);
    let ena = MockPwmOutput::new();
    let in1_h = in1.clone();
    let in2_h = in2.clone();
    let ena_h = ena.clone();
    let ch = L298nChannel::new(in1, in2, ena);
    (in1_h, in2_h, ena_h, ch)
}

#[test]
fn esp32_l298n_channel_forward_sets_in1_high_in2_low() {
    let (in1_h, in2_h, ena_h, mut ch) = make_channel();

    ch.apply(MotorCommand::new(MotorDirection::Forward, 60))
        .unwrap();

    assert!(in1_h.level());
    assert!(!in2_h.level());
    assert_eq!(ena_h.current_duty(), 60);
}

#[test]
fn esp32_l298n_channel_reverse_sets_in1_low_in2_high() {
    let (in1_h, in2_h, ena_h, mut ch) = make_channel();

    ch.apply(MotorCommand::new(MotorDirection::Reverse, 40))
        .unwrap();

    assert!(!in1_h.level());
    assert!(in2_h.level());
    assert_eq!(ena_h.current_duty(), 40);
}

#[test]
fn esp32_l298n_channel_brake_sets_both_pins_high() {
    let (in1_h, in2_h, ena_h, mut ch) = make_channel();

    ch.apply(MotorCommand::new(MotorDirection::Brake, 0))
        .unwrap();

    assert!(in1_h.level());
    assert!(in2_h.level());
    assert_eq!(ena_h.current_duty(), 0);
}

#[test]
fn esp32_l298n_channel_coast_sets_both_pins_low() {
    let (in1_h, in2_h, ena_h, mut ch) = make_channel();

    ch.apply(MotorCommand::new(MotorDirection::Coast, 0))
        .unwrap();

    assert!(!in1_h.level());
    assert!(!in2_h.level());
    assert_eq!(ena_h.current_duty(), 0);
}

#[test]
fn esp32_l298n_channel_rejects_duty_over_100() {
    let (_, _, _, mut ch) = make_channel();
    assert_eq!(
        ch.apply(MotorCommand::new(MotorDirection::Forward, 101)),
        Err(ActuatorError::InvalidCommand)
    );
}

#[test]
fn esp32_l298n_dual_driver_routes_left_and_right_independently() {
    let (in1_a, in2_a, ena_a, ch_a) = make_channel();
    let (in1_b, in2_b, ena_b, ch_b) = make_channel();
    let mut driver = L298nDualDriver::new(ch_a, ch_b);

    driver
        .apply_channels(
            MotorCommand::new(MotorDirection::Forward, 35),
            MotorCommand::new(MotorDirection::Reverse, 20),
        )
        .unwrap();

    // Channel A (left): forward
    assert!(in1_a.level());
    assert!(!in2_a.level());
    assert_eq!(ena_a.current_duty(), 35);

    // Channel B (right): reverse
    assert!(!in1_b.level());
    assert!(in2_b.level());
    assert_eq!(ena_b.current_duty(), 20);
}

#[test]
fn esp32_l298n_dual_driver_records_pwm_history_per_channel() {
    let (_, _, ena_a, ch_a) = make_channel();
    let (_, _, ena_b, ch_b) = make_channel();
    let mut driver = L298nDualDriver::new(ch_a, ch_b);

    driver
        .apply_channels(
            MotorCommand::new(MotorDirection::Forward, 50),
            MotorCommand::new(MotorDirection::Reverse, 30),
        )
        .unwrap();
    driver
        .apply_channels(
            MotorCommand::new(MotorDirection::Brake, 0),
            MotorCommand::new(MotorDirection::Coast, 0),
        )
        .unwrap();

    assert_eq!(ena_a.history(), vec![50, 0]);
    assert_eq!(ena_b.history(), vec![30, 0]);
}
