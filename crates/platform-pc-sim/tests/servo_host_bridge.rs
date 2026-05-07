//! Host bridge test: `ServoDriver<MockPwmOutput>` — sim-to-real path verification.
//!
//! Verifies that the real `ServoDriver` (from `platform-esp32::servo`, re-exported from
//! `reference-drivers`) correctly maps angle commands to PWM duty cycles when backed
//! by a `MockPwmOutput` instead of a real hardware PWM channel.

use hal_api::actuator::ServoMotor;
use platform_esp32::servo::ServoDriver;
use platform_pc_sim::pwm_mock::MockPwmOutput;

/// SERVO_DUTY_MIN = 5% (corresponds to 0°, 1 ms pulse at 50 Hz)
const DUTY_MIN: u8 = 5;
/// SERVO_DUTY_MAX = 10% (corresponds to 180°, 2 ms pulse at 50 Hz)
const DUTY_MAX: u8 = 10;

#[test]
fn esp32_servo_driver_maps_zero_degrees_to_min_duty() {
    let pwm = MockPwmOutput::new();
    let pwm_handle = pwm.clone();
    let mut servo = ServoDriver::new(pwm);

    servo.set_angle_degrees(0).unwrap();

    assert_eq!(pwm_handle.current_duty(), DUTY_MIN);
    assert_eq!(servo.current_angle(), 0);
}

#[test]
fn esp32_servo_driver_maps_180_degrees_to_max_duty() {
    let pwm = MockPwmOutput::new();
    let pwm_handle = pwm.clone();
    let mut servo = ServoDriver::new(pwm);

    servo.set_angle_degrees(180).unwrap();

    assert_eq!(pwm_handle.current_duty(), DUTY_MAX);
    assert_eq!(servo.current_angle(), 180);
}

#[test]
fn esp32_servo_driver_maps_90_degrees_to_midpoint_duty() {
    let pwm = MockPwmOutput::new();
    let pwm_handle = pwm.clone();
    let mut servo = ServoDriver::new(pwm);

    // 90° → 5 + (90 * 5 / 180) = 5 + 2 = 7%
    servo.set_angle_degrees(90).unwrap();

    assert_eq!(pwm_handle.current_duty(), 7);
    assert_eq!(servo.current_angle(), 90);
}

#[test]
fn esp32_servo_driver_rejects_angle_beyond_180() {
    let mut servo = ServoDriver::new(MockPwmOutput::new());
    assert!(servo.set_angle_degrees(181).is_err());
}

#[test]
fn esp32_servo_driver_records_pwm_call_history() {
    let pwm = MockPwmOutput::new();
    let pwm_handle = pwm.clone();
    let mut servo = ServoDriver::new(pwm);

    servo.set_angle_degrees(0).unwrap();
    servo.set_angle_degrees(90).unwrap();
    servo.set_angle_degrees(180).unwrap();

    assert_eq!(pwm_handle.history(), vec![DUTY_MIN, 7, DUTY_MAX]);
    assert_eq!(servo.current_angle(), 180);
}
