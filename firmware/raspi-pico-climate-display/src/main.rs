#![no_std]
#![no_main]

use core::cell::RefCell;
use core::fmt::Write as _;

use cortex_m::delay::Delay;
use embedded_hal::delay::DelayNs;
use fugit::RateExtU32;
use panic_halt as _;
use platform_rp2040::bme280::{
    BME280_ADDRESS_PRIMARY, BME280_ADDRESS_SECONDARY, Bme280Config, Bme280Sensor,
};
use platform_rp2040::i2c::Rp2040I2c;
use platform_rp2040::lcd1602::{LCD1602_ADDRESS_PRIMARY, Lcd1602Config, Lcd1602Display};
use platform_rp2040::shared_i2c::SharedI2cBus;
use rp_pico::entry;
use rp_pico::hal;
use rp_pico::hal::pac;
use rp_pico::hal::Clock;

use core_app::climate_display::{ClimateDisplayApp, ClimateDisplayConfig};
use hal_api::display::TextFrame16x2;

const I2C_SDA_GPIO: u8 = 4;
const I2C_SCL_GPIO: u8 = 5;
const REFRESH_PERIOD_TICKS: u32 = 10;
const LOOP_DELAY_MS: u32 = 100;
const BME280_CHIP_ID_REGISTER: u8 = 0xD0;
const BME280_CHIP_ID_VALUE: u8 = 0x60;

/// `cortex_m::delay::Delay` を `embedded_hal::delay::DelayNs` に橋渡しするラッパー。
struct PicoDelay(Delay);

impl DelayNs for PicoDelay {
    fn delay_ns(&mut self, ns: u32) {
        self.0.delay_us(ns.div_ceil(1000));
    }

    fn delay_us(&mut self, us: u32) {
        self.0.delay_us(us);
    }

    fn delay_ms(&mut self, ms: u32) {
        self.0.delay_ms(ms);
    }
}

/// `RefCell<PicoDelay>` を複数箇所から共有するための薄いラッパー。
/// `SharedI2cBus` と同じパターン。
struct SharedDelay<'a>(&'a RefCell<PicoDelay>);

impl DelayNs for SharedDelay<'_> {
    fn delay_ns(&mut self, ns: u32) {
        self.0.borrow_mut().delay_ns(ns);
    }

    fn delay_us(&mut self, us: u32) {
        self.0.borrow_mut().delay_us(us);
    }

    fn delay_ms(&mut self, ms: u32) {
        self.0.borrow_mut().delay_ms(ms);
    }
}

fn every_nth(value: u32, period: u32) -> bool {
    period != 0 && value % period == 0
}

fn should_log_refresh(tick: u32, config: ClimateDisplayConfig) -> bool {
    (config.refresh_on_first_tick && tick == 1)
        || every_nth(tick, config.refresh_period_ticks.max(1))
}

fn frame_line(frame: &TextFrame16x2, row: usize) -> &str {
    core::str::from_utf8(frame.line(row)).unwrap_or("????????????????")
}

fn detect_bme280_address<B, W>(bus: &mut B, uart: &mut W) -> u8
where
    B: hal_api::i2c::I2cBus<Error = hal_api::error::I2cError>,
    W: core::fmt::Write,
{
    for address in [BME280_ADDRESS_PRIMARY, BME280_ADDRESS_SECONDARY] {
        let mut chip_id = [0u8; 1];
        match bus.write_read(address, &[BME280_CHIP_ID_REGISTER], &mut chip_id) {
            Ok(()) if chip_id[0] == BME280_CHIP_ID_VALUE => {
                let _ = write!(
                    uart,
                    "BME280 probe: detected at 0x{:02x} (chip-id=0x{:02x})\r\n",
                    address, chip_id[0]
                );
                return address;
            }
            Ok(()) => {
                let _ = write!(
                    uart,
                    "BME280 probe: 0x{:02x} unexpected chip-id=0x{:02x}\r\n",
                    address, chip_id[0]
                );
            }
            Err(_) => {
                let _ = write!(uart, "BME280 probe: 0x{:02x} no response\r\n", address);
            }
        }
    }
    let _ = write!(
        uart,
        "BME280 probe: fallback to 0x{:02x}\r\n",
        BME280_ADDRESS_PRIMARY
    );
    BME280_ADDRESS_PRIMARY
}

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();

    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);
    let clocks = hal::clocks::init_clocks_and_plls(
        rp_pico::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let sio = hal::Sio::new(pac.SIO);
    let pins = rp_pico::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // UART0: GPIO0=TX, GPIO1=RX
    let uart_pins = (
        pins.gpio0.into_function::<hal::gpio::FunctionUart>(),
        pins.gpio1.into_function::<hal::gpio::FunctionUart>(),
    );
    let mut uart = hal::uart::UartPeripheral::new(pac.UART0, uart_pins, &mut pac.RESETS)
        .enable(
            hal::uart::UartConfig::new(
                115_200_u32.Hz(),
                hal::uart::DataBits::Eight,
                None,
                hal::uart::StopBits::One,
            ),
            clocks.peripheral_clock.freq(),
        )
        .unwrap();

    // I2C0: GPIO4=SDA, GPIO5=SCL
    let sda = pins.gpio4.into_function::<hal::gpio::FunctionI2C>();
    let scl = pins.gpio5.into_function::<hal::gpio::FunctionI2C>();
    let i2c = hal::I2C::new_controller(
        pac.I2C0,
        sda,
        scl,
        100_000u32.Hz(),
        &mut pac.RESETS,
        clocks.system_clock.freq(),
    );
    let mut rp2040_i2c = Rp2040I2c::new(i2c);
    let bme280_address = detect_bme280_address(&mut rp2040_i2c, &mut uart);
    let shared_bus = RefCell::new(rp2040_i2c);

    let shared_delay = RefCell::new(PicoDelay(Delay::new(
        core.SYST,
        clocks.system_clock.freq().to_Hz(),
    )));

    let app_config = ClimateDisplayConfig {
        refresh_period_ticks: REFRESH_PERIOD_TICKS,
        refresh_on_first_tick: true,
    };

    let sensor = Bme280Sensor::new_with_config(
        SharedI2cBus::new(&shared_bus),
        Bme280Config {
            address: bme280_address,
            ..Bme280Config::default()
        },
    );
    let display = Lcd1602Display::new_with_config(
        SharedI2cBus::new(&shared_bus),
        SharedDelay(&shared_delay),
        Lcd1602Config {
            address: LCD1602_ADDRESS_PRIMARY,
            ..Lcd1602Config::default()
        },
    );
    let mut app = ClimateDisplayApp::new_with_config(sensor, display, app_config);

    let _ = write!(uart, "Raspberry Pi Pico climate display started\r\n");
    let _ = write!(
        uart,
        "I2C: SDA=GPIO{} SCL=GPIO{} BME280=0x{:02x} LCD1602=0x{:02x}\r\n",
        I2C_SDA_GPIO, I2C_SCL_GPIO, bme280_address, LCD1602_ADDRESS_PRIMARY
    );
    let _ = write!(
        uart,
        "refresh: every {} ticks ({} ms loop)\r\n",
        REFRESH_PERIOD_TICKS, LOOP_DELAY_MS
    );

    let mut tick = 0u32;
    loop {
        match app.tick() {
            Ok(()) => {
                tick += 1;
                if should_log_refresh(tick, app_config) {
                    match (app.last_reading(), app.last_frame()) {
                        (Some(reading), Some(frame)) => {
                            let _ = write!(
                                uart,
                                "climate refresh tick={} temp_cc={} hum_cp={} line1=\"{}\" line2=\"{}\"\r\n",
                                tick,
                                reading.temperature_centi_celsius,
                                reading.humidity_centi_percent,
                                frame_line(&frame, 0),
                                frame_line(&frame, 1)
                            );
                        }
                        _ => {
                            let _ = write!(
                                uart,
                                "climate refresh tick={} frame unavailable\r\n",
                                tick
                            );
                        }
                    }
                }
            }
            Err(error) => {
                let _ = write!(uart, "climate tick failed: {:?}\r\n", error);
            }
        }

        shared_delay.borrow_mut().delay_ms(LOOP_DELAY_MS);
    }
}
