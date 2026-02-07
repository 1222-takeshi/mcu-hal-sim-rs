use core_app::App;
use hal_api::gpio::OutputPin;
use hal_api::i2c::I2cBus;
mod mock_hal;
use mock_hal::{MockPin, MockI2c};
use std::thread;
use std::time::Duration;

fn main() {
    println!("=== PC Simulator Started ===");

    let pin = MockPin::new(13); // GPIO13をLEDに見立てる
    let i2c = MockI2c::new();

    let mut app = App::new(pin, i2c);

    loop {
        if let Err(e) = app.tick() {
            eprintln!("Error: {:?}", e);
            break;
        }
        thread::sleep(Duration::from_millis(10)); // 10ms = 100Hz
    }
}
