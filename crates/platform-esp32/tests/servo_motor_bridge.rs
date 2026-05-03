use core::convert::Infallible;

use embedded_hal::digital::{ErrorType as DigitalErrorType, OutputPin as EmbeddedOutputPin};
use embedded_hal::pwm::{ErrorType as PwmErrorType, SetDutyCycle};
use hal_api::actuator::{DriveMotor, DualMotorDriver, MotorCommand, MotorDirection, ServoMotor};
use hal_api::pwm::PwmOutput;
use platform_esp32::gpio::Esp32OutputPin;
use platform_esp32::l298n::{L298nChannel, L298nDualDriver};
use platform_esp32::pwm::Esp32PwmOutput;
use platform_esp32::servo::ServoDriver;

// ---------------------------------------------------------------------------
// Dummy embedded-hal GPIO pin
// ---------------------------------------------------------------------------

struct DummyOutputPin {
    level: bool,
    call_count: usize,
}

impl DummyOutputPin {
    fn new() -> Self {
        Self {
            level: false,
            call_count: 0,
        }
    }
}

impl DigitalErrorType for DummyOutputPin {
    type Error = Infallible;
}

impl EmbeddedOutputPin for DummyOutputPin {
    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.level = true;
        self.call_count += 1;
        Ok(())
    }

    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.level = false;
        self.call_count += 1;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Dummy embedded-hal PWM channel
// ---------------------------------------------------------------------------

struct DummyPwmChannel {
    duty: u16,
    max: u16,
}

impl DummyPwmChannel {
    fn new(max: u16) -> Self {
        Self { duty: 0, max }
    }
}

impl PwmErrorType for DummyPwmChannel {
    type Error = Infallible;
}

impl SetDutyCycle for DummyPwmChannel {
    fn max_duty_cycle(&self) -> u16 {
        self.max
    }

    fn set_duty_cycle(&mut self, duty: u16) -> Result<(), Self::Error> {
        self.duty = duty;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helper to build an ESP32-backed servo driver
// ---------------------------------------------------------------------------

fn make_servo() -> ServoDriver<Esp32PwmOutput<DummyPwmChannel>> {
    ServoDriver::new(Esp32PwmOutput::new(DummyPwmChannel::new(1000)))
}

// ---------------------------------------------------------------------------
// Helper to build one L298N channel with ESP32 adapters
// ---------------------------------------------------------------------------

type Esp32Channel = L298nChannel<
    Esp32OutputPin<DummyOutputPin>,
    Esp32OutputPin<DummyOutputPin>,
    Esp32PwmOutput<DummyPwmChannel>,
>;

fn make_channel() -> Esp32Channel {
    L298nChannel::new(
        Esp32OutputPin::new(DummyOutputPin::new()),
        Esp32OutputPin::new(DummyOutputPin::new()),
        Esp32PwmOutput::new(DummyPwmChannel::new(1000)),
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn servo_driver_works_with_esp32_pwm_adapter() {
    let mut servo = make_servo();

    servo.set_angle_degrees(0).unwrap();
    assert_eq!(servo.current_angle(), 0);
    // 0° → duty 5% of max 1000 → 50
    assert_eq!(servo.pwm().duty_percent(), 5);

    servo.set_angle_degrees(90).unwrap();
    assert_eq!(servo.current_angle(), 90);

    servo.set_angle_degrees(180).unwrap();
    assert_eq!(servo.current_angle(), 180);
    // 180° → duty 10% of max 1000 → 100
    assert_eq!(servo.pwm().duty_percent(), 10);
}

#[test]
fn servo_driver_rejects_angle_beyond_180_via_esp32_adapter() {
    let mut servo = make_servo();
    assert!(servo.set_angle_degrees(181).is_err());
}

#[test]
fn l298n_dual_driver_works_with_esp32_adapters() {
    let mut driver = L298nDualDriver::new(make_channel(), make_channel());

    let left = MotorCommand::new(MotorDirection::Forward, 60);
    let right = MotorCommand::new(MotorDirection::Reverse, 40);

    driver.apply_channels(left, right).unwrap();

    assert_eq!(
        driver.channel_a().current_command().direction,
        MotorDirection::Forward
    );
    assert_eq!(driver.channel_a().current_command().duty_percent, 60);
    assert_eq!(
        driver.channel_b().current_command().direction,
        MotorDirection::Reverse
    );
    assert_eq!(driver.channel_b().current_command().duty_percent, 40);
}

#[test]
fn l298n_channel_forward_drives_gpio_pins_correctly_via_esp32() {
    let mut ch = make_channel();
    ch.apply(MotorCommand::new(MotorDirection::Forward, 75))
        .unwrap();

    // IN1 = HIGH, IN2 = LOW for Forward
    assert!(ch.in1().inner().level);
    assert!(!ch.in2().inner().level);
    assert_eq!(ch.enable().duty_percent(), 75);
}

#[test]
fn l298n_channel_brake_drives_both_pins_high_via_esp32() {
    let mut ch = make_channel();
    ch.apply(MotorCommand::new(MotorDirection::Brake, 0))
        .unwrap();

    assert!(ch.in1().inner().level);
    assert!(ch.in2().inner().level);
}

#[test]
fn full_actuator_scenario_servo_tracks_distance_motor_responds_to_direction() {
    let mut servo = make_servo();
    let mut driver = L298nDualDriver::new(make_channel(), make_channel());

    // Simulate: object at 200mm → servo 70°, motors forward
    let distance_mm: u32 = 200;
    let angle = ((distance_mm.saturating_sub(80)) * 180 / 280) as u16;
    servo.set_angle_degrees(angle.min(180)).unwrap();

    let cmd = MotorCommand::new(MotorDirection::Forward, 42);
    driver.apply_channels(cmd, cmd).unwrap();

    assert!(servo.current_angle() <= 180);
    assert_eq!(
        driver.channel_a().current_command().direction,
        MotorDirection::Forward
    );
    assert_eq!(
        driver.channel_b().current_command().direction,
        MotorDirection::Forward
    );
}
