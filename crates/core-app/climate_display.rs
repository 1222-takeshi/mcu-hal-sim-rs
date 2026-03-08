use core::fmt::Write as _;

use hal_api::display::TextDisplay16x2;
use hal_api::display::TextFrame16x2;
use hal_api::error::{DisplayError, SensorError};
use hal_api::sensor::{EnvReading, EnvSensor};
use heapless::String;

#[cfg(test)]
extern crate std;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClimateDisplayConfig {
    pub refresh_period_ticks: u32,
}

impl Default for ClimateDisplayConfig {
    fn default() -> Self {
        Self {
            refresh_period_ticks: 100,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ClimateDisplayError {
    Sensor(SensorError),
    Display(DisplayError),
}

impl From<SensorError> for ClimateDisplayError {
    fn from(error: SensorError) -> Self {
        Self::Sensor(error)
    }
}

impl From<DisplayError> for ClimateDisplayError {
    fn from(error: DisplayError) -> Self {
        Self::Display(error)
    }
}

pub struct ClimateDisplayApp<SENSOR, DISPLAY> {
    sensor: SENSOR,
    display: DISPLAY,
    tick_count: u32,
    config: ClimateDisplayConfig,
    last_reading: Option<EnvReading>,
    last_frame: Option<TextFrame16x2>,
}

impl<SENSOR, DISPLAY> ClimateDisplayApp<SENSOR, DISPLAY>
where
    SENSOR: EnvSensor<Error = SensorError>,
    DISPLAY: TextDisplay16x2<Error = DisplayError>,
{
    pub fn new(sensor: SENSOR, display: DISPLAY) -> Self {
        Self::new_with_config(sensor, display, ClimateDisplayConfig::default())
    }

    pub fn new_with_config(sensor: SENSOR, display: DISPLAY, config: ClimateDisplayConfig) -> Self {
        Self {
            sensor,
            display,
            tick_count: 0,
            config,
            last_reading: None,
            last_frame: None,
        }
    }

    pub fn tick(&mut self) -> Result<(), ClimateDisplayError> {
        self.tick_count += 1;

        if self.should_refresh() {
            self.refresh()?;
        }

        Ok(())
    }

    pub fn refresh(&mut self) -> Result<(), ClimateDisplayError> {
        let reading = self.sensor.read()?;
        let frame = frame_from_reading(reading)?;
        self.display.render(&frame)?;
        self.last_reading = Some(reading);
        self.last_frame = Some(frame);
        Ok(())
    }

    fn should_refresh(&self) -> bool {
        if self.tick_count == 1 {
            return true;
        }

        let period = self.config.refresh_period_ticks.max(1);
        self.tick_count.checked_rem(period) == Some(0)
    }

    #[cfg(test)]
    pub fn tick_count(&self) -> u32 {
        self.tick_count
    }

    #[cfg(test)]
    pub fn last_reading(&self) -> Option<EnvReading> {
        self.last_reading
    }

    #[cfg(test)]
    pub fn last_frame(&self) -> Option<TextFrame16x2> {
        self.last_frame
    }
}

pub fn frame_from_reading(reading: EnvReading) -> Result<TextFrame16x2, DisplayError> {
    let mut line1: String<17> = String::new();
    let mut line2: String<17> = String::new();

    write_temperature(&mut line1, reading.temperature_centi_celsius)
        .map_err(|_| DisplayError::InvalidContent)?;
    write_humidity(&mut line2, reading.humidity_centi_percent)
        .map_err(|_| DisplayError::InvalidContent)?;

    Ok(TextFrame16x2::from_lines(&line1, &line2))
}

fn write_temperature(line: &mut String<17>, temperature_centi_celsius: i32) -> core::fmt::Result {
    let temperature_tenths = if temperature_centi_celsius >= 0 {
        (temperature_centi_celsius + 5) / 10
    } else {
        (temperature_centi_celsius - 5) / 10
    };
    let whole = temperature_tenths / 10;
    let tenth = temperature_tenths.abs() % 10;
    write!(line, "Temp {:>5}.{}C", whole, tenth)
}

fn write_humidity(line: &mut String<17>, humidity_centi_percent: u32) -> core::fmt::Result {
    let humidity_tenths = (humidity_centi_percent + 5) / 10;
    let whole = humidity_tenths / 10;
    let tenth = humidity_tenths % 10;
    write!(line, "Hum  {:>5}.{}%", whole, tenth)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hal_api::display::TextFrame16x2;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Clone)]
    struct TestSensor {
        reading: EnvReading,
        reads: Rc<RefCell<u32>>,
    }

    impl TestSensor {
        fn new(reading: EnvReading) -> Self {
            Self {
                reading,
                reads: Rc::new(RefCell::new(0)),
            }
        }

        fn read_count(&self) -> u32 {
            *self.reads.borrow()
        }
    }

    impl EnvSensor for TestSensor {
        type Error = SensorError;

        fn read(&mut self) -> Result<EnvReading, Self::Error> {
            *self.reads.borrow_mut() += 1;
            Ok(self.reading)
        }
    }

    #[derive(Clone)]
    struct TestDisplay {
        frames: Rc<RefCell<std::vec::Vec<TextFrame16x2>>>,
    }

    impl TestDisplay {
        fn new() -> Self {
            Self {
                frames: Rc::new(RefCell::new(std::vec::Vec::new())),
            }
        }

        fn frames(&self) -> std::vec::Vec<TextFrame16x2> {
            self.frames.borrow().clone()
        }
    }

    impl TextDisplay16x2 for TestDisplay {
        type Error = DisplayError;

        fn render(&mut self, frame: &TextFrame16x2) -> Result<(), Self::Error> {
            self.frames.borrow_mut().push(*frame);
            Ok(())
        }
    }

    fn line_to_string(frame: &TextFrame16x2, row: usize) -> String<17> {
        let mut output = String::<17>::new();
        for byte in frame.line(row) {
            let _ = output.push(*byte as char);
        }
        output
    }

    #[test]
    fn climate_display_app_renders_on_first_tick() {
        let sensor = TestSensor::new(EnvReading::new(2481, 4315, None));
        let sensor_observer = sensor.clone();
        let display = TestDisplay::new();
        let display_observer = display.clone();
        let mut app = ClimateDisplayApp::new(sensor, display);

        app.tick().unwrap();

        assert_eq!(sensor_observer.read_count(), 1);
        assert_eq!(display_observer.frames().len(), 1);
        assert_eq!(app.tick_count(), 1);
    }

    #[test]
    fn climate_display_app_respects_refresh_period() {
        let sensor = TestSensor::new(EnvReading::new(2481, 4315, None));
        let sensor_observer = sensor.clone();
        let display = TestDisplay::new();
        let mut app = ClimateDisplayApp::new_with_config(
            sensor,
            display,
            ClimateDisplayConfig {
                refresh_period_ticks: 5,
            },
        );

        for _ in 0..9 {
            app.tick().unwrap();
        }

        assert_eq!(sensor_observer.read_count(), 2);
    }

    #[test]
    fn frame_from_reading_formats_temperature_and_humidity() {
        let frame = frame_from_reading(EnvReading::new(2481, 4315, None)).unwrap();

        assert_eq!(line_to_string(&frame, 0), "Temp    24.8C   ");
        assert_eq!(line_to_string(&frame, 1), "Hum     43.2%   ");
    }

    #[test]
    fn frame_from_reading_handles_negative_temperature() {
        let frame = frame_from_reading(EnvReading::new(-520, 8000, None)).unwrap();

        assert_eq!(line_to_string(&frame, 0), "Temp    -5.2C   ");
        assert_eq!(line_to_string(&frame, 1), "Hum     80.0%   ");
    }

    #[test]
    fn climate_display_app_treats_zero_refresh_period_as_every_tick() {
        let sensor = TestSensor::new(EnvReading::new(2481, 4315, None));
        let sensor_observer = sensor.clone();
        let display = TestDisplay::new();
        let display_observer = display.clone();
        let mut app = ClimateDisplayApp::new_with_config(
            sensor,
            display,
            ClimateDisplayConfig {
                refresh_period_ticks: 0,
            },
        );

        for _ in 0..3 {
            app.tick().unwrap();
        }

        assert_eq!(sensor_observer.read_count(), 3);
        assert_eq!(display_observer.frames().len(), 3);
    }

    #[test]
    fn frame_from_reading_rounds_to_nearest_tenth() {
        let frame = frame_from_reading(EnvReading::new(2495, 994, None)).unwrap();

        assert_eq!(line_to_string(&frame, 0), "Temp    25.0C   ");
        assert_eq!(line_to_string(&frame, 1), "Hum      9.9%   ");
    }

    #[test]
    fn frame_from_reading_handles_three_digit_temperature_and_humidity() {
        let frame = frame_from_reading(EnvReading::new(12345, 10000, None)).unwrap();

        assert_eq!(line_to_string(&frame, 0), "Temp   123.5C   ");
        assert_eq!(line_to_string(&frame, 1), "Hum    100.0%   ");
    }

    #[test]
    fn frame_from_reading_rounds_negative_temperature_away_from_zero() {
        let frame = frame_from_reading(EnvReading::new(-525, 8000, None)).unwrap();

        assert_eq!(line_to_string(&frame, 0), "Temp    -5.3C   ");
        assert_eq!(line_to_string(&frame, 1), "Hum     80.0%   ");
    }

    #[test]
    fn frame_from_reading_handles_maximum_humidity_rounding() {
        let frame = frame_from_reading(EnvReading::new(2481, 9995, None)).unwrap();

        assert_eq!(line_to_string(&frame, 0), "Temp    24.8C   ");
        assert_eq!(line_to_string(&frame, 1), "Hum    100.0%   ");
    }
}
