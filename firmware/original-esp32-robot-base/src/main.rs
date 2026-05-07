//! # original-esp32-robot-base
//!
//! ESP32 ロボットベース ファームウェアスケルトン。
//! `Esp32ServoDriver` と `Esp32L298nDualDriverSimple` を使ったサーボ + デュアルモータ制御の
//! 実機接続例です。
//!
//! ## 配線
//!
//! ### サーボモータ（SG90 等）
//!
//! | 信号 | ESP32 GPIO | 備考 |
//! |------|-----------|------|
//! | PWM  | GPIO 18   | LEDC チャンネル 0, 50 Hz |
//!
//! ### L298N デュアルモータドライバ
//!
//! | 信号    | ESP32 GPIO | 備考 |
//! |--------|-----------|------|
//! | IN1-A  | GPIO 25   | チャンネル A 方向 1 |
//! | IN2-A  | GPIO 26   | チャンネル A 方向 2 |
//! | ENA    | GPIO 27   | チャンネル A PWM (LEDC Ch1) |
//! | IN1-B  | GPIO 32   | チャンネル B 方向 1 |
//! | IN2-B  | GPIO 33   | チャンネル B 方向 2 |
//! | ENB    | GPIO 14   | チャンネル B PWM (LEDC Ch2) |
//!
//! ## ビルド・書き込み
//!
//! ```bash
//! # xtensa ツールチェーン（espup install 済みであること）
//! cd firmware/original-esp32-robot-base
//! cargo build --release
//! cargo run --release   # espflash でフラッシュ + シリアルモニタ
//! ```
#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{
    gpio::{Level, Output, OutputConfig},
    ledc::{
        channel::{ChannelConfig, ChannelIFace},
        timer::{TimerConfig, TimerIFace},
        LSGlobalClkSource, Ledc, LowSpeed,
    },
    main,
    time::{Duration, Instant},
};
use esp_println::println;
use hal_api::actuator::{DriveMotor, DualMotorDriver, MotorCommand, MotorDirection, ServoMotor};
use platform_esp32::{
    gpio::Esp32OutputPin,
    pwm::Esp32PwmOutput,
    types::{Esp32L298nChannel, Esp32L298nDualDriverSimple, Esp32ServoDriver},
};

esp_bootloader_esp_idf::esp_app_desc!();

// ── ピン番号定数 ──────────────────────────────────────────────────────────────
// NOTE: これらの定数は println! によるログ出力用です。
//       実際のペリフェラル初期化は peripherals.GPIOxx を直接使用します。
//       ピン変更時は両方を同時に更新してください。
const SERVO_PWM_GPIO: u8 = 18;
const MOTOR_IN1_A_GPIO: u8 = 25;
const MOTOR_IN2_A_GPIO: u8 = 26;
const MOTOR_ENA_GPIO: u8 = 27;
const MOTOR_IN1_B_GPIO: u8 = 32;
const MOTOR_IN2_B_GPIO: u8 = 33;
const MOTOR_ENB_GPIO: u8 = 14;

// ── タイミング定数 ────────────────────────────────────────────────────────────
/// メインループの周期（1 tick = 20 ms）
const LOOP_PERIOD_MS: u32 = 20;
/// デモシーケンスの 1 ステップあたりの tick 数
const DEMO_STEP_TICKS: u32 = 50; // 50 × 20 ms = 1 s

// ── 型エイリアス（GPIO/LEDC の具体型を指定） ─────────────────────────────────
// NOTE: `esp_hal::gpio::Output` は `embedded_hal::digital::OutputPin` を実装しており、
// `esp_hal::ledc::channel::Channel` は `embedded_hal::pwm::SetDutyCycle` を実装している。
// `Esp32OutputPin<T>` / `Esp32PwmOutput<P>` がそれぞれのラッパーとなる。
type EspOutput = Output<'static>;
type EspLedcChannel<'d> = esp_hal::ledc::channel::Channel<'d, LowSpeed>;

// ── デモシーケンス定義 ────────────────────────────────────────────────────────
#[derive(Clone, Copy)]
struct DemoStep {
    servo_angle: u16,
    motor_duty: u8,
    direction: MotorDirection,
}

static DEMO_SEQUENCE: &[DemoStep] = &[
    DemoStep { servo_angle: 0,   motor_duty: 50, direction: MotorDirection::Forward },
    DemoStep { servo_angle: 45,  motor_duty: 75, direction: MotorDirection::Forward },
    DemoStep { servo_angle: 90,  motor_duty: 0,  direction: MotorDirection::Brake },
    DemoStep { servo_angle: 135, motor_duty: 60, direction: MotorDirection::Reverse },
    DemoStep { servo_angle: 180, motor_duty: 40, direction: MotorDirection::Reverse },
    DemoStep { servo_angle: 90,  motor_duty: 0,  direction: MotorDirection::Brake },
];

fn monotonic_delay_ms(ms: u32) {
    let target = Duration::from_millis(ms as u64);
    let start = Instant::now();
    while start.elapsed() < target {}
}

#[main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    // ── GPIO 出力ピンの初期化 ────────────────────────────────────────────────
    let in1_a: Output<'static> =
        Output::new(peripherals.GPIO25, Level::Low, OutputConfig::default());
    let in2_a: Output<'static> =
        Output::new(peripherals.GPIO26, Level::Low, OutputConfig::default());
    let in1_b: Output<'static> =
        Output::new(peripherals.GPIO32, Level::Low, OutputConfig::default());
    let in2_b: Output<'static> =
        Output::new(peripherals.GPIO33, Level::Low, OutputConfig::default());

    // ── LEDC（PWM）の初期化 ─────────────────────────────────────────────────
    let mut ledc = Ledc::new(peripherals.LEDC);
    ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);

    let mut servo_timer = ledc.timer::<LowSpeed>(esp_hal::ledc::timer::Number::Timer0);
    servo_timer
        .configure(TimerConfig {
            duty: esp_hal::ledc::timer::config::Duty::Duty14Bit,
            clock_source: esp_hal::ledc::timer::LSClockSource::APBClk,
            frequency: esp_hal::time::Rate::from_hz(50), // 50 Hz for servo
        })
        .unwrap();

    let mut motor_timer = ledc.timer::<LowSpeed>(esp_hal::ledc::timer::Number::Timer1);
    motor_timer
        .configure(TimerConfig {
            duty: esp_hal::ledc::timer::config::Duty::Duty8Bit,
            clock_source: esp_hal::ledc::timer::LSClockSource::APBClk,
            frequency: esp_hal::time::Rate::from_hz(1_000), // 1 kHz for motors
        })
        .unwrap();

    let mut servo_ch = ledc.channel(
        esp_hal::ledc::channel::Number::Channel0,
        peripherals.GPIO18,
    );
    servo_ch
        .configure(ChannelConfig {
            timer: &servo_timer,
            duty_pct: 0,
            pin_config: esp_hal::ledc::channel::config::PinConfig::PushPull,
        })
        .unwrap();

    let mut ena_ch = ledc.channel(
        esp_hal::ledc::channel::Number::Channel1,
        peripherals.GPIO27,
    );
    ena_ch
        .configure(ChannelConfig {
            timer: &motor_timer,
            duty_pct: 0,
            pin_config: esp_hal::ledc::channel::config::PinConfig::PushPull,
        })
        .unwrap();

    let mut enb_ch = ledc.channel(
        esp_hal::ledc::channel::Number::Channel2,
        peripherals.GPIO14,
    );
    enb_ch
        .configure(ChannelConfig {
            timer: &motor_timer,
            duty_pct: 0,
            pin_config: esp_hal::ledc::channel::config::PinConfig::PushPull,
        })
        .unwrap();

    // ── platform-esp32 ラッパーへの接続 ─────────────────────────────────────
    let servo_pwm = Esp32PwmOutput::new(servo_ch);
    let mut servo: Esp32ServoDriver<EspLedcChannel<'_>> = Esp32ServoDriver::new(servo_pwm);

    let ch_a = Esp32L298nChannel::new(
        Esp32OutputPin::new(in1_a),
        Esp32OutputPin::new(in2_a),
        Esp32PwmOutput::new(ena_ch),
    );
    let ch_b = Esp32L298nChannel::new(
        Esp32OutputPin::new(in1_b),
        Esp32OutputPin::new(in2_b),
        Esp32PwmOutput::new(enb_ch),
    );
    let mut motors: Esp32L298nDualDriverSimple<EspOutput, EspLedcChannel<'_>> =
        Esp32L298nDualDriverSimple::new(ch_a, ch_b);

    println!("=== ESP32 Robot Base Firmware ===");
    println!("Servo: GPIO {}", SERVO_PWM_GPIO);
    println!("Motor A: IN1={} IN2={} ENA={}", MOTOR_IN1_A_GPIO, MOTOR_IN2_A_GPIO, MOTOR_ENA_GPIO);
    println!("Motor B: IN1={} IN2={} ENB={}", MOTOR_IN1_B_GPIO, MOTOR_IN2_B_GPIO, MOTOR_ENB_GPIO);

    // ── メインループ ─────────────────────────────────────────────────────────
    let mut tick: u32 = 0;
    loop {
        let step_index =
            ((tick / DEMO_STEP_TICKS) as usize) % DEMO_SEQUENCE.len();
        let step = &DEMO_SEQUENCE[step_index];

        if tick % DEMO_STEP_TICKS == 0 {
            println!(
                "[tick {}] step {} — servo={}° motor={:?}@{}%",
                tick, step_index, step.servo_angle, step.direction, step.motor_duty
            );

            if let Err(e) = servo.set_angle_degrees(step.servo_angle) {
                println!("servo error: {:?}", e);
            }

            let cmd = MotorCommand::new(step.direction, step.motor_duty);
            if let Err(e) = motors.apply_channels(cmd, cmd) {
                println!("motor error: {:?}", e);
            }
        }

        tick = tick.wrapping_add(1);
        monotonic_delay_ms(LOOP_PERIOD_MS);
    }
}
