#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{
    gpio::{Input, InputConfig, Pull},
    i2c::master::{Config as I2cConfig, I2c},
    main,
};
use esp_println::println;
use hal_api::error::{GpioError, I2cError};
use hal_api::gpio::InputPin;
use hal_api::i2c::I2cBus;
use m5stickc_bringup::report::ProbeSummary;
use platform_esp32::{gpio::Esp32InputPin, i2c::Esp32I2c};

esp_bootloader_esp_idf::esp_app_desc!();

const BUTTON_A_GPIO: u8 = 37;
const BUTTON_B_GPIO: u8 = 39;
const I2C_SDA_GPIO: u8 = 21;
const I2C_SCL_GPIO: u8 = 22;
const LOOP_DELAY_SPINS: u32 = 6_000_000;
const BUS_RECHECK_PERIOD_LOOPS: u32 = 32;

const AXP192_ADDRESS: u8 = 0x34;
const AXP192_STATUS_REGISTER: u8 = 0x00;
const BM8563_ADDRESS: u8 = 0x51;
const BM8563_CONTROL1_REGISTER: u8 = 0x00;
const MPU6886_ADDRESS: u8 = 0x68;
const MPU6886_WHO_AM_I_REGISTER: u8 = 0x75;
const SH200Q_ADDRESS: u8 = 0x6c;
const SH200Q_WHO_AM_I_REGISTER: u8 = 0x30;
const SH200Q_WHO_AM_I_VALUE: u8 = 0x18;
const BME280_PRIMARY_ADDRESS: u8 = 0x76;
const BME280_SECONDARY_ADDRESS: u8 = 0x77;
const BME280_CHIP_ID_REGISTER: u8 = 0xD0;
const BME280_CHIP_ID_VALUE: u8 = 0x60;
const HEARTBEAT_PERIOD_LOOPS: u32 = 40;

fn every_nth(value: u32, period: u32) -> bool {
    period != 0 && value % period == 0
}

fn busy_wait(iterations: u32) {
    for _ in 0..iterations {
        core::hint::spin_loop();
    }
}

fn probe_register<I>(
    i2c: &mut I,
    device_name: &str,
    addr: u8,
    register: u8,
    buffer: &mut [u8],
) -> bool
where
    I: I2cBus<Error = I2cError>,
{
    match i2c.write_read(addr, &[register], buffer) {
        Ok(()) => {
            println!(
                "probe: {} responded at 0x{:02x}, reg 0x{:02x} -> {:02x?}",
                device_name, addr, register, buffer
            );
            true
        }
        Err(I2cError::InvalidAddress) => {
            println!("probe: {} not found at 0x{:02x}", device_name, addr);
            false
        }
        Err(error) => {
            println!(
                "probe: {} read failed at 0x{:02x}: {:?}",
                device_name, addr, error
            );
            false
        }
    }
}

fn probe_mpu6886<I>(i2c: &mut I) -> bool
where
    I: I2cBus<Error = I2cError>,
{
    let mut who_am_i = [0u8; 1];
    probe_register(
        i2c,
        "MPU6886",
        MPU6886_ADDRESS,
        MPU6886_WHO_AM_I_REGISTER,
        &mut who_am_i,
    )
}

fn probe_sh200q<I>(i2c: &mut I) -> bool
where
    I: I2cBus<Error = I2cError>,
{
    let mut who_am_i = [0u8; 1];
    match i2c.write_read(SH200Q_ADDRESS, &[SH200Q_WHO_AM_I_REGISTER], &mut who_am_i) {
        Ok(()) if who_am_i[0] == SH200Q_WHO_AM_I_VALUE => {
            println!(
                "probe: SH200Q detected at 0x{:02x} (chip-id=0x{:02x})",
                SH200Q_ADDRESS, who_am_i[0]
            );
            true
        }
        Ok(()) => {
            println!(
                "probe: SH200Q candidate responded at 0x{:02x}, chip-id=0x{:02x}",
                SH200Q_ADDRESS, who_am_i[0]
            );
            true
        }
        Err(I2cError::InvalidAddress) => {
            println!("probe: SH200Q not found at 0x{:02x}", SH200Q_ADDRESS);
            false
        }
        Err(error) => {
            println!(
                "probe: SH200Q read failed at 0x{:02x}: {:?}",
                SH200Q_ADDRESS, error
            );
            false
        }
    }
}

fn read_pressed<P>(button_name: &str, button: &P) -> Result<bool, GpioError>
where
    P: InputPin<Error = GpioError>,
{
    button.is_low().inspect_err(|error| {
        println!("button: {} read failed: {:?}", button_name, error);
    })
}

fn button_state_label(pressed: bool) -> &'static str {
    if pressed { "pressed" } else { "released" }
}

fn button_combo_label(button_a_pressed: bool, button_b_pressed: bool) -> &'static str {
    match (button_a_pressed, button_b_pressed) {
        (false, false) => "idle",
        (true, false) => "A only",
        (false, true) => "B only",
        (true, true) => "A+B",
    }
}

fn probe_bme280<I>(i2c: &mut I, addr: u8) -> bool
where
    I: I2cBus<Error = I2cError>,
{
    let mut chip_id = [0u8; 1];
    match i2c.write_read(addr, &[BME280_CHIP_ID_REGISTER], &mut chip_id) {
        Ok(()) if chip_id[0] == BME280_CHIP_ID_VALUE => {
            println!(
                "probe: BME280 detected at 0x{:02x} (chip-id=0x{:02x})",
                addr, chip_id[0]
            );
            true
        }
        Ok(()) => {
            println!(
                "probe: BME280 candidate responded at 0x{:02x}, chip-id=0x{:02x}",
                addr, chip_id[0]
            );
            true
        }
        Err(I2cError::InvalidAddress) => {
            println!("probe: BME280 not found at 0x{:02x}", addr);
            false
        }
        Err(error) => {
            println!("probe: BME280 read failed at 0x{:02x}: {:?}", addr, error);
            false
        }
    }
}

fn log_bus_health<I>(i2c: &mut I, loop_count: u32)
where
    I: I2cBus<Error = I2cError>,
{
    let mut axp192 = [0u8; 1];
    match i2c.write_read(AXP192_ADDRESS, &[AXP192_STATUS_REGISTER], &mut axp192) {
        Ok(()) => println!(
            "bus health: loop={} AXP192 ack reg0=0x{:02x}",
            loop_count, axp192[0]
        ),
        Err(I2cError::InvalidAddress) => println!(
            "bus health: loop={} AXP192 missing at 0x{:02x}",
            loop_count, AXP192_ADDRESS
        ),
        Err(error) => println!(
            "bus health: loop={} AXP192 probe failed: {:?}",
            loop_count, error
        ),
    }
}

#[main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    let button_a = Input::new(
        peripherals.GPIO37,
        InputConfig::default().with_pull(Pull::Up),
    );
    let button_a = Esp32InputPin::new(button_a);

    let button_b = Input::new(
        peripherals.GPIO39,
        InputConfig::default().with_pull(Pull::Up),
    );
    let button_b = Esp32InputPin::new(button_b);

    let bus = I2c::new(peripherals.I2C0, I2cConfig::default())
        .unwrap()
        .with_sda(peripherals.GPIO21)
        .with_scl(peripherals.GPIO22);
    let mut i2c = Esp32I2c::new(bus);

    println!("M5StickC bring-up started");
    println!(
        "Button A = GPIO{}, Button B = GPIO{}",
        BUTTON_A_GPIO, BUTTON_B_GPIO
    );
    println!(
        "I2C: SDA = GPIO{}, SCL = GPIO{}",
        I2C_SDA_GPIO, I2C_SCL_GPIO
    );
    println!(
        "loop timing: sample_delay_spins={} heartbeat_every={} bus_recheck_every={}",
        LOOP_DELAY_SPINS, HEARTBEAT_PERIOD_LOOPS, BUS_RECHECK_PERIOD_LOOPS
    );
    println!("LED GPIO10 is present on the board, but esp-hal does not expose it on esp32");
    println!("probe: checking common onboard I2C devices");

    let mut axp192 = [0u8; 1];
    let axp192_found = probe_register(
        &mut i2c,
        "AXP192",
        AXP192_ADDRESS,
        AXP192_STATUS_REGISTER,
        &mut axp192,
    );

    let mut bm8563 = [0u8; 2];
    let bm8563_found = probe_register(
        &mut i2c,
        "BM8563",
        BM8563_ADDRESS,
        BM8563_CONTROL1_REGISTER,
        &mut bm8563,
    );

    let mpu6886_found = probe_mpu6886(&mut i2c);
    let sh200q_found = probe_sh200q(&mut i2c);
    let bme280_primary_found = probe_bme280(&mut i2c, BME280_PRIMARY_ADDRESS);
    let bme280_secondary_found = probe_bme280(&mut i2c, BME280_SECONDARY_ADDRESS);
    let summary = ProbeSummary {
        axp192_found,
        bm8563_found,
        mpu6886_found,
        sh200q_found,
        bme280_primary_found,
        bme280_secondary_found,
    };

    println!(
        "probe summary: AXP192={} BM8563={} MPU6886={} SH200Q={} BME280@0x76={} BME280@0x77={}",
        if summary.axp192_found { "yes" } else { "no" },
        if summary.bm8563_found { "yes" } else { "no" },
        if summary.mpu6886_found { "yes" } else { "no" },
        if summary.sh200q_found { "yes" } else { "no" },
        if summary.bme280_primary_found {
            "yes"
        } else {
            "no"
        },
        if summary.bme280_secondary_found {
            "yes"
        } else {
            "no"
        },
    );
    println!(
        "i2c scan summary: [0x34:{}] [0x51:{}] [0x68:{}] [0x6c:{}] [0x76:{}] [0x77:{}]",
        if summary.axp192_found { "ACK" } else { "--" },
        if summary.bm8563_found { "ACK" } else { "--" },
        if summary.mpu6886_found { "ACK" } else { "--" },
        if summary.sh200q_found { "ACK" } else { "--" },
        if summary.bme280_primary_found {
            "ACK"
        } else {
            "--"
        },
        if summary.bme280_secondary_found {
            "ACK"
        } else {
            "--"
        },
    );
    println!(
        "board status: pmu_rtc={} imu={} external_bme280={} onboard_i2c={}",
        summary.pmu_rtc_status(),
        summary.imu_status(),
        summary.external_bme280_status(),
        if summary.onboard_i2c_alive() {
            "alive"
        } else {
            "missing"
        },
    );
    println!("board hint: {}", summary.health_hint());

    let mut loop_count = 0u32;
    let mut button_a_pressed = read_pressed("A", &button_a).unwrap_or(false);
    let mut button_b_pressed = read_pressed("B", &button_b).unwrap_or(false);
    let mut button_combo = button_combo_label(button_a_pressed, button_b_pressed);

    println!("button: A initial {}", button_state_label(button_a_pressed));
    println!("button: B initial {}", button_state_label(button_b_pressed));
    println!("button: combo initial {}", button_combo);

    loop {
        let previous_button_a = button_a_pressed;
        let previous_button_b = button_b_pressed;

        match read_pressed("A", &button_a) {
            Ok(pressed) if pressed != button_a_pressed => {
                button_a_pressed = pressed;
                println!(
                    "button: A {} (loop={})",
                    button_state_label(button_a_pressed),
                    loop_count
                );
            }
            Ok(_) => {}
            Err(_) => {}
        }

        match read_pressed("B", &button_b) {
            Ok(pressed) if pressed != button_b_pressed => {
                button_b_pressed = pressed;
                println!(
                    "button: B {} (loop={})",
                    button_state_label(button_b_pressed),
                    loop_count
                );
            }
            Ok(_) => {}
            Err(_) => {}
        }

        let combo_changed =
            previous_button_a != button_a_pressed || previous_button_b != button_b_pressed;
        let next_combo = button_combo_label(button_a_pressed, button_b_pressed);
        if combo_changed && next_combo != button_combo {
            button_combo = next_combo;
            println!("button: combo {} (loop={})", button_combo, loop_count);
        }

        loop_count += 1;
        if every_nth(loop_count, HEARTBEAT_PERIOD_LOOPS) {
            println!(
                "heartbeat loop = {} (A={} B={} combo={} imu={} ext_bme280={})",
                loop_count,
                button_state_label(button_a_pressed),
                button_state_label(button_b_pressed),
                button_combo,
                summary.imu_status(),
                summary.external_bme280_status(),
            );
        }
        if every_nth(loop_count, BUS_RECHECK_PERIOD_LOOPS) {
            log_bus_health(&mut i2c, loop_count);
        }

        busy_wait(LOOP_DELAY_SPINS);
    }
}
