//! HC-SR04 ultrasonic distance sensor driver.

use hal_api::distance::{DistanceReading, DistanceSensor, UltrasonicPulseDevice};
use hal_api::error::SensorError;

const SPEED_OF_SOUND_NUMERATOR: u32 = 343;
const SPEED_OF_SOUND_DENOMINATOR: u32 = 2000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HcSr04Config {
    pub min_echo_us: u32,
    pub max_echo_us: u32,
}

impl Default for HcSr04Config {
    fn default() -> Self {
        Self {
            min_echo_us: 100,
            max_echo_us: 25_000,
        }
    }
}

/// HC-SR04 driver.
pub struct HcSr04Sensor<D> {
    device: D,
    config: HcSr04Config,
}

impl<D> HcSr04Sensor<D> {
    pub fn new(device: D) -> Self {
        Self::new_with_config(device, HcSr04Config::default())
    }

    pub fn new_with_config(device: D, config: HcSr04Config) -> Self {
        Self { device, config }
    }

    pub fn config(&self) -> HcSr04Config {
        self.config
    }
}

impl<D> DistanceSensor for HcSr04Sensor<D>
where
    D: UltrasonicPulseDevice<Error = SensorError>,
{
    type Error = SensorError;

    fn read_distance(&mut self) -> Result<DistanceReading, Self::Error> {
        let echo_us = self.device.trigger_and_measure_echo_us()?;
        if echo_us < self.config.min_echo_us || echo_us > self.config.max_echo_us {
            return Err(SensorError::InvalidReading);
        }

        Ok(DistanceReading::new(echo_us_to_distance_mm(echo_us)))
    }
}

fn echo_us_to_distance_mm(echo_us: u32) -> u32 {
    (echo_us * SPEED_OF_SOUND_NUMERATOR) / SPEED_OF_SOUND_DENOMINATOR
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate std;
    use self::std::cell::RefCell;
    use self::std::rc::Rc;

    #[derive(Clone)]
    struct RecordingPulseDevice {
        state: Rc<RefCell<RecordingPulseState>>,
    }

    struct RecordingPulseState {
        next_echo_us: Result<u32, SensorError>,
        calls: usize,
    }

    impl RecordingPulseDevice {
        fn new(next_echo_us: Result<u32, SensorError>) -> Self {
            Self {
                state: Rc::new(RefCell::new(RecordingPulseState {
                    next_echo_us,
                    calls: 0,
                })),
            }
        }

        fn call_count(&self) -> usize {
            self.state.borrow().calls
        }
    }

    impl UltrasonicPulseDevice for RecordingPulseDevice {
        type Error = SensorError;

        fn trigger_and_measure_echo_us(&mut self) -> Result<u32, Self::Error> {
            let mut state = self.state.borrow_mut();
            state.calls += 1;
            state.next_echo_us.clone()
        }
    }

    #[test]
    fn hc_sr04_sensor_converts_echo_time_to_distance() {
        let device = RecordingPulseDevice::new(Ok(1_050));
        let mut sensor = HcSr04Sensor::new(device.clone());

        let reading = sensor.read_distance().unwrap();

        assert_eq!(reading.distance_mm, 180);
        assert_eq!(device.call_count(), 1);
    }

    #[test]
    fn hc_sr04_sensor_rejects_out_of_range_echo() {
        let device = RecordingPulseDevice::new(Ok(30_000));
        let mut sensor = HcSr04Sensor::new(device);

        assert_eq!(sensor.read_distance(), Err(SensorError::InvalidReading));
    }

    #[test]
    fn hc_sr04_sensor_propagates_measurement_errors() {
        let device = RecordingPulseDevice::new(Err(SensorError::Busy));
        let mut sensor = HcSr04Sensor::new(device);

        assert_eq!(sensor.read_distance(), Err(SensorError::Busy));
    }
}
