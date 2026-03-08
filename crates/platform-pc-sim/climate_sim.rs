//! Climate display simulator helpers.

use core_app::climate_display::{ClimateDisplayApp, ClimateDisplayConfig};
use hal_api::display::{TextDisplay16x2, TextFrame16x2};
use hal_api::error::{DisplayError, SensorError};
use hal_api::sensor::{EnvReading, EnvSensor};
use std::cell::RefCell;
use std::fmt::Write as _;
use std::rc::Rc;
use std::string::String;
use std::vec::Vec;

#[derive(Debug, Default)]
struct SimulatedEnvState {
    readings: Vec<EnvReading>,
    next_index: usize,
    read_count: usize,
    loop_forever: bool,
}

/// シーケンス化された温湿度センサシミュレータ。
#[derive(Clone, Debug, Default)]
pub struct SimulatedEnvSensor {
    state: Rc<RefCell<SimulatedEnvState>>,
}

pub type SequenceEnvSensor = SimulatedEnvSensor;

impl SimulatedEnvSensor {
    pub fn new(readings: Vec<EnvReading>) -> Self {
        Self {
            state: Rc::new(RefCell::new(SimulatedEnvState {
                readings,
                next_index: 0,
                read_count: 0,
                loop_forever: false,
            })),
        }
    }

    pub fn looping(readings: Vec<EnvReading>) -> Self {
        let sensor = Self::new(readings);
        sensor.state.borrow_mut().loop_forever = true;
        sensor
    }

    pub fn read_count(&self) -> usize {
        self.state.borrow().read_count
    }

    pub fn current_index(&self) -> usize {
        self.state.borrow().next_index
    }
}

impl EnvSensor for SimulatedEnvSensor {
    type Error = SensorError;

    fn read(&mut self) -> Result<EnvReading, Self::Error> {
        let mut state = self.state.borrow_mut();
        if state.readings.is_empty() {
            return Err(SensorError::NotInitialized);
        }

        let reading = state.readings[state.next_index];
        state.read_count += 1;

        if state.loop_forever {
            state.next_index = (state.next_index + 1) % state.readings.len();
        } else if state.next_index + 1 < state.readings.len() {
            state.next_index += 1;
        }

        Ok(reading)
    }
}

#[derive(Debug, Default)]
struct TerminalDisplayState {
    last_frame: Option<TextFrame16x2>,
    render_count: usize,
    render_to_stdout: bool,
}

/// 16x2 LCD を terminal ASCII へ描画する表示シミュレータ。
#[derive(Clone, Debug, Default)]
pub struct TerminalDisplay16x2 {
    state: Rc<RefCell<TerminalDisplayState>>,
}

impl TerminalDisplay16x2 {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_stdout() -> Self {
        let display = Self::default();
        display.state.borrow_mut().render_to_stdout = true;
        display
    }

    pub fn render_count(&self) -> usize {
        self.state.borrow().render_count
    }

    pub fn last_frame(&self) -> Option<TextFrame16x2> {
        self.state.borrow().last_frame
    }

    pub fn last_ascii(&self) -> Option<String> {
        self.last_frame().map(|frame| render_ascii_frame(&frame))
    }
}

impl TextDisplay16x2 for TerminalDisplay16x2 {
    type Error = DisplayError;

    fn render(&mut self, frame: &TextFrame16x2) -> Result<(), Self::Error> {
        let mut state = self.state.borrow_mut();
        state.last_frame = Some(*frame);
        state.render_count += 1;

        if state.render_to_stdout {
            print!("\x1B[2J\x1B[H{}", render_ascii_frame(frame));
        }

        Ok(())
    }
}

pub fn render_ascii_frame(frame: &TextFrame16x2) -> String {
    let mut output = String::new();
    let _ = writeln!(output, "+----------------+");

    for row in 0..2 {
        let line = frame.line(row);
        let _ = write!(output, "|");
        for byte in line {
            output.push(*byte as char);
        }
        let _ = writeln!(output, "|");
    }

    let _ = writeln!(output, "+----------------+");
    output
}

pub fn demo_sensor_readings() -> Vec<EnvReading> {
    vec![
        EnvReading::new(2480, 4310, Some(101_325)),
        EnvReading::new(2510, 4380, Some(101_280)),
        EnvReading::new(2570, 4460, Some(101_240)),
        EnvReading::new(2620, 4520, Some(101_210)),
    ]
}

pub fn build_demo_app(
    sensor: SimulatedEnvSensor,
    display: TerminalDisplay16x2,
) -> ClimateDisplayApp<SimulatedEnvSensor, TerminalDisplay16x2> {
    ClimateDisplayApp::new_with_config(
        sensor,
        display,
        ClimateDisplayConfig {
            refresh_period_ticks: 5,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simulated_env_sensor_holds_last_value_by_default() {
        let readings = vec![
            EnvReading::new(2480, 4310, None),
            EnvReading::new(2520, 4400, None),
        ];
        let mut sensor = SimulatedEnvSensor::new(readings);

        assert_eq!(sensor.read().unwrap(), EnvReading::new(2480, 4310, None));
        assert_eq!(sensor.read().unwrap(), EnvReading::new(2520, 4400, None));
        assert_eq!(sensor.read().unwrap(), EnvReading::new(2520, 4400, None));
        assert_eq!(sensor.read_count(), 3);
    }

    #[test]
    fn simulated_env_sensor_loops_when_requested() {
        let readings = vec![
            EnvReading::new(2480, 4310, None),
            EnvReading::new(2520, 4400, None),
        ];
        let mut sensor = SimulatedEnvSensor::looping(readings);

        assert_eq!(sensor.read().unwrap(), EnvReading::new(2480, 4310, None));
        assert_eq!(sensor.read().unwrap(), EnvReading::new(2520, 4400, None));
        assert_eq!(sensor.read().unwrap(), EnvReading::new(2480, 4310, None));
    }

    #[test]
    fn terminal_display_renders_frame_as_ascii() {
        let frame = TextFrame16x2::from_lines("Temp    24.8C", "Hum     43.2%");
        let ascii = render_ascii_frame(&frame);

        assert_eq!(
            ascii,
            "+----------------+\n|Temp    24.8C   |\n|Hum     43.2%   |\n+----------------+\n"
        );
    }

    #[test]
    fn climate_display_app_updates_terminal_display() {
        let sensor = SimulatedEnvSensor::new(vec![EnvReading::new(2480, 4310, Some(101_325))]);
        let sensor_observer = sensor.clone();
        let display = TerminalDisplay16x2::new();
        let display_observer = display.clone();
        let mut app = build_demo_app(sensor, display);

        for _ in 0..5 {
            app.tick().unwrap();
        }

        assert_eq!(sensor_observer.read_count(), 2);
        assert_eq!(display_observer.render_count(), 2);
        assert_eq!(
            display_observer.last_ascii().unwrap(),
            "+----------------+\n|Temp    24.8C   |\n|Hum     43.1%   |\n+----------------+\n"
        );
    }
}
