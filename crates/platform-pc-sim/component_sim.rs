//! Host-side simulators for distance / IMU / actuator components.

use hal_api::actuator::{DriveMotor, DualMotorDriver, MotorCommand, ServoMotor};
use hal_api::distance::{DistanceReading, DistanceSensor};
use hal_api::error::{ActuatorError, SensorError};
use hal_api::imu::{ImuReading, ImuSensor};
use std::cell::RefCell;
use std::rc::Rc;
use std::vec::Vec;

#[derive(Debug, Default)]
struct SequenceState<T> {
    values: Vec<T>,
    next_index: usize,
    read_count: usize,
    loop_forever: bool,
}

fn next_sequence_value<T: Copy>(state: &mut SequenceState<T>) -> Option<T> {
    let value = *state.values.get(state.next_index)?;
    state.read_count += 1;

    if state.loop_forever {
        state.next_index = (state.next_index + 1) % state.values.len();
    } else if state.next_index + 1 < state.values.len() {
        state.next_index += 1;
    }

    Some(value)
}

#[derive(Clone, Debug)]
pub struct SequenceDistanceSensor {
    state: Rc<RefCell<SequenceState<DistanceReading>>>,
}

impl SequenceDistanceSensor {
    pub fn new(values: Vec<DistanceReading>) -> Self {
        Self {
            state: Rc::new(RefCell::new(SequenceState {
                values,
                next_index: 0,
                read_count: 0,
                loop_forever: false,
            })),
        }
    }

    pub fn looping(values: Vec<DistanceReading>) -> Self {
        let sensor = Self::new(values);
        sensor.state.borrow_mut().loop_forever = true;
        sensor
    }

    pub fn read_count(&self) -> usize {
        self.state.borrow().read_count
    }
}

impl DistanceSensor for SequenceDistanceSensor {
    type Error = SensorError;

    fn read_distance(&mut self) -> Result<DistanceReading, Self::Error> {
        next_sequence_value(&mut self.state.borrow_mut()).ok_or(SensorError::NotInitialized)
    }
}

#[derive(Clone, Debug)]
pub struct SequenceImuSensor {
    state: Rc<RefCell<SequenceState<ImuReading>>>,
}

impl SequenceImuSensor {
    pub fn new(values: Vec<ImuReading>) -> Self {
        Self {
            state: Rc::new(RefCell::new(SequenceState {
                values,
                next_index: 0,
                read_count: 0,
                loop_forever: false,
            })),
        }
    }

    pub fn looping(values: Vec<ImuReading>) -> Self {
        let sensor = Self::new(values);
        sensor.state.borrow_mut().loop_forever = true;
        sensor
    }

    pub fn read_count(&self) -> usize {
        self.state.borrow().read_count
    }
}

impl ImuSensor for SequenceImuSensor {
    type Error = SensorError;

    fn read_imu(&mut self) -> Result<ImuReading, Self::Error> {
        next_sequence_value(&mut self.state.borrow_mut()).ok_or(SensorError::NotInitialized)
    }
}

#[derive(Debug, Default)]
struct ServoState {
    angle_degrees: u16,
}

#[derive(Clone, Debug, Default)]
pub struct MockServoMotor {
    state: Rc<RefCell<ServoState>>,
}

impl MockServoMotor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn angle_degrees(&self) -> u16 {
        self.state.borrow().angle_degrees
    }
}

impl ServoMotor for MockServoMotor {
    type Error = ActuatorError;

    fn set_angle_degrees(&mut self, angle_degrees: u16) -> Result<(), Self::Error> {
        if angle_degrees > 180 {
            return Err(ActuatorError::InvalidCommand);
        }
        self.state.borrow_mut().angle_degrees = angle_degrees;
        Ok(())
    }
}

#[derive(Debug)]
struct MotorState {
    left: MotorCommand,
    right: MotorCommand,
}

impl Default for MotorState {
    fn default() -> Self {
        Self {
            left: MotorCommand::new(hal_api::actuator::MotorDirection::Coast, 0),
            right: MotorCommand::new(hal_api::actuator::MotorDirection::Coast, 0),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct MockDualMotorDriver {
    state: Rc<RefCell<MotorState>>,
}

impl MockDualMotorDriver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn left_command(&self) -> MotorCommand {
        self.state.borrow().left
    }

    pub fn right_command(&self) -> MotorCommand {
        self.state.borrow().right
    }
}

impl DriveMotor for MockDualMotorDriver {
    type Error = ActuatorError;

    // This single-channel apply sets BOTH channels to the same command.
    // This is a mock convenience for simulation — real dual-motor drivers
    // would expose per-channel control via DualMotorDriver::apply_channels.
    fn apply(&mut self, command: MotorCommand) -> Result<(), Self::Error> {
        if command.duty_percent > 100 {
            return Err(ActuatorError::InvalidCommand);
        }

        let mut state = self.state.borrow_mut();
        state.left = command;
        state.right = command;
        Ok(())
    }
}

impl DualMotorDriver for MockDualMotorDriver {
    type Error = ActuatorError;

    fn apply_channels(
        &mut self,
        left: MotorCommand,
        right: MotorCommand,
    ) -> Result<(), Self::Error> {
        if left.duty_percent > 100 || right.duty_percent > 100 {
            return Err(ActuatorError::InvalidCommand);
        }

        let mut state = self.state.borrow_mut();
        state.left = left;
        state.right = right;
        Ok(())
    }
}

pub fn demo_distance_readings() -> Vec<DistanceReading> {
    vec![
        DistanceReading::new(180),
        DistanceReading::new(240),
        DistanceReading::new(320),
        DistanceReading::new(140),
    ]
}

pub fn demo_imu_readings() -> Vec<ImuReading> {
    vec![
        ImuReading::new([0, 0, 1_000], [0, 0, 0], Some(2_450)),
        ImuReading::new([120, -40, 980], [0, 320, 0], Some(2_460)),
        ImuReading::new([-160, 90, 1_020], [0, -280, 40], Some(2_470)),
        ImuReading::new([40, 140, 990], [120, 0, -80], Some(2_465)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use hal_api::actuator::{MotorDirection, ServoMotor};
    use hal_api::distance::DistanceSensor;
    use hal_api::imu::ImuSensor;

    #[test]
    fn sequence_distance_sensor_loops() {
        let mut sensor = SequenceDistanceSensor::looping(vec![
            DistanceReading::new(100),
            DistanceReading::new(200),
        ]);

        assert_eq!(sensor.read_distance().unwrap().distance_mm, 100);
        assert_eq!(sensor.read_distance().unwrap().distance_mm, 200);
        assert_eq!(sensor.read_distance().unwrap().distance_mm, 100);
    }

    #[test]
    fn sequence_imu_sensor_returns_values() {
        let expected = ImuReading::new([1, 2, 3], [4, 5, 6], Some(123));
        let mut sensor = SequenceImuSensor::new(vec![expected]);

        assert_eq!(sensor.read_imu().unwrap(), expected);
    }

    #[test]
    fn mock_servo_motor_rejects_invalid_angle() {
        let mut servo = MockServoMotor::new();

        assert_eq!(
            servo.set_angle_degrees(181),
            Err(ActuatorError::InvalidCommand)
        );
        servo.set_angle_degrees(90).unwrap();
        assert_eq!(servo.angle_degrees(), 90);
    }

    #[test]
    fn mock_dual_motor_driver_tracks_channels() {
        let mut driver = MockDualMotorDriver::new();
        let left = MotorCommand::new(MotorDirection::Forward, 35);
        let right = MotorCommand::new(MotorDirection::Reverse, 20);

        driver.apply_channels(left, right).unwrap();

        assert_eq!(driver.left_command(), left);
        assert_eq!(driver.right_command(), right);
    }
}
