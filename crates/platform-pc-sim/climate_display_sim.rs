//! Terminal demo for `ClimateDisplayApp`.

use core_app::climate_display::{ClimateDisplayApp, ClimateDisplayConfig};
use platform_pc_sim::climate_sim::{demo_sensor_readings, SequenceEnvSensor, TerminalDisplay16x2};
use std::thread;
use std::time::Duration;

fn main() {
    println!("=== Climate Display Sim ===");
    println!("Ctrl+C で終了します");

    let sensor = SequenceEnvSensor::looping(demo_sensor_readings());
    let display = TerminalDisplay16x2::with_stdout();
    let mut app = ClimateDisplayApp::new_with_config(
        sensor,
        display,
        ClimateDisplayConfig {
            refresh_period_ticks: 5,
            refresh_on_first_tick: true,
        },
    );

    loop {
        if let Err(error) = app.tick() {
            eprintln!("climate sim failed: {:?}", error);
            break;
        }
        thread::sleep(Duration::from_millis(200));
    }
}
