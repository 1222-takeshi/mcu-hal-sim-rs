#![no_std]
#![no_main]

use cortex_m::delay::Delay;
use embedded_hal::i2c::I2c as _;
use fugit::RateExtU32;
use panic_halt as _;
use platform_rp2040::{gpio::Rp2040OutputPin, i2c::Rp2040I2c};
use rp_pico::entry;
use rp_pico::hal;
use rp_pico::hal::pac;
use rp_pico::hal::Clock;

use core_app::App;

const I2C_FREQUENCY_HZ: u32 = 100_000;
const HEARTBEAT_DELAY_MS: u32 = 250;

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
    let mut i2c = hal::I2C::new_controller(
        pac.I2C0,
        sda,
        scl,
        I2C_FREQUENCY_HZ.Hz(),
        &mut pac.RESETS,
        clocks.system_clock.freq(),
    );

    // LED: GPIO25 (onboard)
    let led = pins.led.into_push_pull_output();

    let mut delay = Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    uart.write_full_blocking(b"raspi-pico bring-up + hal-api demo\r\n");
    uart.write_full_blocking(b"LED=GPIO25 SDA=GPIO4 SCL=GPIO5 UART0=GPIO0/1\r\n");
    uart.write_full_blocking(
        b"Use this firmware to confirm blink/UART/I2C before adding sensors.\r\n",
    );

    // I2C scan (0x08..=0x77)
    uart.write_full_blocking(b"I2C scan:\r\n");
    for addr in 0x08u8..=0x77 {
        let mut buf = [0u8; 1];
        if i2c.read(addr, &mut buf).is_ok() {
            uart.write_full_blocking(b"  found device at 0x");
            uart.write_full_blocking(&[hex_nibble(addr >> 4), hex_nibble(addr & 0x0F)]);
            uart.write_full_blocking(b"\r\n");
        }
    }
    uart.write_full_blocking(b"I2C scan done.\r\n");

    // Wrap into hal-api adapters and run core-app
    let rp2040_pin = Rp2040OutputPin::new(led);
    let rp2040_i2c = Rp2040I2c::new(i2c);
    let mut app = App::new(rp2040_pin, rp2040_i2c);

    uart.write_full_blocking(b"Starting core-app via hal-api adapters...\r\n");

    let mut heartbeat = 0u32;
    loop {
        heartbeat = heartbeat.wrapping_add(1);

        if app.tick().is_err() {
            uart.write_full_blocking(b"app.tick() error\r\n");
        }

        if heartbeat % 4 == 0 {
            uart.write_full_blocking(b"heartbeat\r\n");
        }

        delay.delay_ms(HEARTBEAT_DELAY_MS);
    }
}

fn hex_nibble(n: u8) -> u8 {
    if n < 10 {
        b'0' + n
    } else {
        b'a' + n - 10
    }
}
