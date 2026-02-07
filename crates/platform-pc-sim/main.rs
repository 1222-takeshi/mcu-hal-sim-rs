use core_app::App;
mod mock_hal;
use mock_hal::{MockI2c, MockPin};
use std::thread;
use std::time::Duration;

fn main() {
    println!("=== PC Simulator Started ===");

    let pin = MockPin::new(13);
    let i2c = MockI2c::new();
    let mut app = App::new(pin, i2c);

    loop {
        if let Err(e) = app.tick() {
            eprintln!("Error: {:?}", e);
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
}

