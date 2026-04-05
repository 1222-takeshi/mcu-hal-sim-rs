//! Rich terminal dashboard for the reference climate path.

use core_app::climate_display::{ClimateDisplayApp, ClimateDisplayConfig};
use embedded_hal::delay::DelayNs;
use platform_pc_sim::bme280_mock::{demo_raw_samples, MockBme280Device};
use platform_pc_sim::dashboard::{render_dashboard, BoardProfile, DashboardSnapshot};
use platform_pc_sim::lcd1602_mock::MockLcd1602Device;
use platform_pc_sim::virtual_i2c::VirtualI2cBus;
use reference_drivers::bme280::{Bme280Sensor, BME280_ADDRESS_PRIMARY};
use reference_drivers::lcd1602::{Lcd1602Display, LCD1602_ADDRESS_PRIMARY};
use std::env;
use std::thread;
use std::time::Duration;

const REFRESH_PERIOD_TICKS: u32 = 5;
const LOOP_DELAY_MS: u64 = 200;

#[derive(Default)]
struct NoopDelay;

impl DelayNs for NoopDelay {
    fn delay_ns(&mut self, _ns: u32) {}
    fn delay_us(&mut self, _us: u32) {}
    fn delay_ms(&mut self, _ms: u32) {}
}

fn main() {
    let board = BoardProfile::from_arg(env::args().nth(1).as_deref());
    let bus = VirtualI2cBus::new();
    let bme280 = MockBme280Device::new();
    let lcd = MockLcd1602Device::new();
    bus.attach_device(BME280_ADDRESS_PRIMARY, bme280.clone());
    bus.attach_device(LCD1602_ADDRESS_PRIMARY, lcd.clone());

    let sensor = Bme280Sensor::new(bus.clone());
    let display = Lcd1602Display::new(bus.clone(), NoopDelay);
    let app_config = ClimateDisplayConfig {
        refresh_period_ticks: REFRESH_PERIOD_TICKS,
        refresh_on_first_tick: true,
    };
    let mut app = ClimateDisplayApp::new_with_config(sensor, display, app_config);

    let samples = demo_raw_samples();
    let mut sample_index = 0usize;

    loop {
        bme280.set_raw_sample(samples[sample_index]);
        sample_index = (sample_index + 1) % samples.len();

        if let Err(error) = app.tick() {
            eprintln!("dashboard sim failed: {:?}", error);
            break;
        }

        let attached_addresses = bus.attached_addresses();
        let operations = bus.operations();
        let snapshot = DashboardSnapshot {
            board,
            tick: app.tick_count(),
            refresh_period_ticks: app.config().refresh_period_ticks,
            reading: app.last_reading(),
            rendered_frame: app.last_frame(),
            physical_frame: lcd.frame(),
            bme280_registers: bme280.control_registers(),
            bme280_raw_sample: bme280.raw_sample(),
            lcd_initialized: lcd.is_initialized(),
            lcd_backlight: lcd.backlight_enabled(),
            attached_addresses: &attached_addresses,
            operations: &operations,
        };

        print!("{}", render_dashboard(&snapshot));
        thread::sleep(Duration::from_millis(LOOP_DELAY_MS));
    }
}
