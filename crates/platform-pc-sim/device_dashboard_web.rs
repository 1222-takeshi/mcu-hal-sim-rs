use core::str;
use std::env;
use std::fmt::Write as FmtWrite;
use std::io::{self, Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use core_app::climate_display::{ClimateDisplayApp, ClimateDisplayConfig};
use embedded_hal::delay::DelayNs;
use hal_api::actuator::{DualMotorDriver, MotorCommand, MotorDirection, ServoMotor};
use hal_api::camera::CameraCapture;
use hal_api::distance::DistanceSensor;
use hal_api::gas::GasSensor;
use hal_api::imu::ImuSensor;
use hal_api::light::LightSensor;
use hal_api::rtc::RtcSensor;
use platform_pc_sim::bme280_mock::{demo_raw_samples, MockBme280Device};
use platform_pc_sim::camera_mock::MockCamera;
use platform_pc_sim::dashboard::BoardProfile;
use platform_pc_sim::ds3231_mock::{demo_timestamps, MockDs3231Device};
use platform_pc_sim::hc_sr04_mock::{demo_echo_pulses_us, MockHcSr04Device};
use platform_pc_sim::lcd1602_mock::MockLcd1602Device;
use platform_pc_sim::mock_hal::MockPin;
use platform_pc_sim::mpu6050_mock::{demo_raw_frames, MockMpu6050Device};
use platform_pc_sim::pwm_mock::MockPwmOutput;
use platform_pc_sim::sgp30_mock::MockSgp30Device;
use platform_pc_sim::virtual_i2c::{VirtualI2cBus, VirtualI2cOperation};
use platform_pc_sim::vl53l0x_mock::MockVl53l0xDevice;
use platform_pc_sim::web_dashboard::{
    dashboard_html, state_to_json, CameraPanelState, ClimatePanelState, DeviceDashboardState,
    DistancePanelState, GasPanelState, I2cPanelState, ImuPanelState, LightPanelState,
    MotorChannelState, MotorDriverPanelState, RtcPanelState, ServoPanelState, TofPanelState,
    WiringPanelState,
};
use platform_pc_sim::wiring_config::{SensorProfile, WiringConfig};
use platform_pc_sim::wiring_svg::wiring_svg;
use reference_drivers::bh1750::{Bh1750Sensor, BH1750_ADDRESS_LOW};
use reference_drivers::bme280::{Bme280Sensor, BME280_ADDRESS_PRIMARY};
use reference_drivers::ds3231::{Ds3231Sensor, DS3231_ADDRESS};
use reference_drivers::hc_sr04::HcSr04Sensor;
use reference_drivers::l298n::{L298nChannel, L298nDualDriver};
use reference_drivers::lcd1602::{Lcd1602Display, LCD1602_ADDRESS_PRIMARY};
use reference_drivers::mpu6050::{Mpu6050Sensor, MPU6050_ADDRESS_PRIMARY};
use reference_drivers::servo::ServoDriver;
use reference_drivers::sgp30::{Sgp30Sensor, SGP30_ADDRESS};
use reference_drivers::vl53l0x::{Vl53l0xSensor, VL53L0X_ADDRESS};

const DEFAULT_PORT: u16 = 7878;

/// Shared server state passed to every connection-handler thread.
struct ServerContext {
    latest_json: Mutex<String>,
    ws_clients: Mutex<Vec<mpsc::SyncSender<String>>>,
    current_board: Mutex<BoardProfile>,
    current_sensor_profile: Mutex<SensorProfile>,
    /// Last wiring-editor JSON submitted via POST /api/wiring/editor.
    editor_json: Mutex<String>,
}

impl ServerContext {
    fn new(board: BoardProfile) -> Arc<Self> {
        Arc::new(Self {
            latest_json: Mutex::new("{}".into()),
            ws_clients: Mutex::new(vec![]),
            current_board: Mutex::new(board),
            current_sensor_profile: Mutex::new(SensorProfile::Full),
            editor_json: Mutex::new("{}".into()),
        })
    }

    fn push_state(&self, json: String) {
        *self.latest_json.lock().unwrap() = json.clone();
        self.ws_clients
            .lock()
            .unwrap()
            .retain(|tx| tx.try_send(json.clone()).is_ok());
    }
}

#[derive(Default)]
struct NoopDelay;

impl DelayNs for NoopDelay {
    fn delay_ns(&mut self, _ns: u32) {}
    fn delay_us(&mut self, _us: u32) {}
    fn delay_ms(&mut self, _ms: u32) {}
}

type ServoRig = ServoDriver<MockPwmOutput>;
type MotorChannelRig = L298nChannel<MockPin, MockPin, MockPwmOutput>;
type MotorDriverRig = L298nDualDriver<MotorChannelRig, MotorChannelRig>;

struct DeviceSimulationRig {
    board: BoardProfile,
    bus: VirtualI2cBus,
    bme280: MockBme280Device,
    lcd: MockLcd1602Device,
    mpu6050: MockMpu6050Device,
    app: ClimateDisplayApp<Bme280Sensor<VirtualI2cBus>, Lcd1602Display<VirtualI2cBus, NoopDelay>>,
    bme280_samples: Vec<[u8; 8]>,
    bme280_sample_index: usize,
    distance_sensor: HcSr04Sensor<MockHcSr04Device>,
    imu_sensor: Mpu6050Sensor<VirtualI2cBus>,
    imu_frames: Vec<[u8; 14]>,
    imu_frame_index: usize,
    servo: ServoRig,
    motor_driver: MotorDriverRig,
    last_distance_mm: Option<u32>,
    last_imu: Option<hal_api::imu::ImuReading>,
    light_sensor: Bh1750Sensor<VirtualI2cBus>,
    camera: MockCamera,
    last_lux_x100: u32,
    last_camera_sequence: u32,
    // New sensors: SGP30, DS3231, VL53L0X
    ds3231_mock: MockDs3231Device,
    ds3231_timestamps: Vec<platform_pc_sim::ds3231_mock::MockRtcTimestamp>,
    ds3231_ts_index: usize,
    rtc_sensor: Ds3231Sensor<VirtualI2cBus>,
    sgp30_sensor: Sgp30Sensor<VirtualI2cBus>,
    tof_sensor: Vl53l0xSensor<VirtualI2cBus>,
    last_gas: Option<hal_api::gas::GasReading>,
    last_rtc_str: String,
    last_tof_mm: Option<u32>,
}

impl DeviceSimulationRig {
    fn new(board: BoardProfile) -> Self {
        let bus = VirtualI2cBus::new();
        let bme280 = MockBme280Device::new();
        let lcd = MockLcd1602Device::new();
        let mpu6050 = MockMpu6050Device::new();
        let bh1750 = platform_pc_sim::bh1750_mock::MockBh1750Device::looping(vec![
            8_500, 12_000, 15_300, 9_800, 7_200,
        ]);
        // DS3231 shares 0x68 with MPU6050 in real hardware, but in simulation both coexist.
        // We register DS3231 at a separate internal address 0x68 — the VirtualI2cBus routes
        // independently; MPU6050 is already attached at 0x68 via mpu6050.clone(), so we use
        // 0x69 (alt address) for DS3231 in the simulation to avoid collision.
        let ds3231_mock = MockDs3231Device::new();
        let sgp30_mock = MockSgp30Device::new();
        let vl53l0x_mock = MockVl53l0xDevice::new();

        bus.attach_device(BME280_ADDRESS_PRIMARY, bme280.clone());
        bus.attach_device(LCD1602_ADDRESS_PRIMARY, lcd.clone());
        bus.attach_device(MPU6050_ADDRESS_PRIMARY, mpu6050.clone());
        bus.attach_device(BH1750_ADDRESS_LOW, bh1750);
        // DS3231 at 0x69 (alt sim address to avoid MPU6050 collision at 0x68)
        bus.attach_device(DS3231_ADDRESS + 1, ds3231_mock.clone());
        bus.attach_device(SGP30_ADDRESS, sgp30_mock.clone());
        bus.attach_device(VL53L0X_ADDRESS, vl53l0x_mock.clone());

        let app = ClimateDisplayApp::new_with_config(
            Bme280Sensor::new(bus.clone()),
            Lcd1602Display::new(bus.clone(), NoopDelay),
            ClimateDisplayConfig {
                refresh_period_ticks: 5,
                refresh_on_first_tick: true,
            },
        );
        let distance_sensor = HcSr04Sensor::new(MockHcSr04Device::looping(demo_echo_pulses_us()));
        let imu_sensor = Mpu6050Sensor::new(bus.clone());
        let light_sensor = Bh1750Sensor::new(bus.clone(), BH1750_ADDRESS_LOW)
            .expect("BH1750 mock device should initialise");
        let camera = MockCamera::qvga_jpeg();
        let rtc_sensor = Ds3231Sensor::new(bus.clone(), DS3231_ADDRESS + 1);
        let sgp30_sensor =
            Sgp30Sensor::new(bus.clone(), SGP30_ADDRESS).expect("SGP30 mock init should succeed");
        let tof_sensor = Vl53l0xSensor::new(bus.clone(), VL53L0X_ADDRESS)
            .expect("VL53L0X mock init should succeed");

        let servo = ServoDriver::new(MockPwmOutput::new());
        let motor_driver = L298nDualDriver::new(
            L298nChannel::new(MockPin::new(0), MockPin::new(0), MockPwmOutput::new()),
            L298nChannel::new(MockPin::new(0), MockPin::new(0), MockPwmOutput::new()),
        );

        Self {
            board,
            bus,
            bme280,
            lcd,
            mpu6050,
            app,
            bme280_samples: demo_raw_samples(),
            bme280_sample_index: 0,
            distance_sensor,
            imu_sensor,
            imu_frames: demo_raw_frames(),
            imu_frame_index: 0,
            servo,
            motor_driver,
            last_distance_mm: None,
            last_imu: None,
            light_sensor,
            camera,
            last_lux_x100: 0,
            last_camera_sequence: 0,
            ds3231_mock,
            ds3231_timestamps: demo_timestamps(),
            ds3231_ts_index: 0,
            rtc_sensor,
            sgp30_sensor,
            tof_sensor,
            last_gas: None,
            last_rtc_str: String::new(),
            last_tof_mm: None,
        }
    }

    fn step(&mut self) -> DeviceDashboardState {
        self.bme280
            .set_raw_sample(self.bme280_samples[self.bme280_sample_index]);
        self.bme280_sample_index = (self.bme280_sample_index + 1) % self.bme280_samples.len();
        self.mpu6050
            .set_raw_frame(self.imu_frames[self.imu_frame_index]);
        self.imu_frame_index = (self.imu_frame_index + 1) % self.imu_frames.len();

        self.app
            .tick()
            .expect("dashboard climate app should keep running");
        let tick = self.app.tick_count();

        if tick == 1 || tick % 2 == 0 {
            self.last_distance_mm = Some(
                self.distance_sensor
                    .read_distance()
                    .expect("distance driver should read from host-side pulse device")
                    .distance_mm,
            );
        }

        if tick == 1 || tick % 3 == 0 {
            self.last_imu = Some(
                self.imu_sensor
                    .read_imu()
                    .expect("imu driver should read from host-side mock device"),
            );
        }

        if tick == 1 || tick % 5 == 0 {
            if let Ok(reading) = self.light_sensor.read_lux() {
                self.last_lux_x100 = reading.lux_x100;
            }
        }

        if tick == 1 || tick % 7 == 0 {
            if let Ok(frame) = self.camera.capture_frame() {
                self.last_camera_sequence = frame.sequence;
            }
        }

        // Poll SGP30 gas sensor every 11 ticks
        if tick == 1 || tick % 11 == 0 {
            if let Ok(reading) = self.sgp30_sensor.read_gas() {
                self.last_gas = Some(reading);
            }
        }

        // Poll DS3231 RTC every 13 ticks; also advance the mock timestamp
        if tick == 1 || tick % 13 == 0 {
            let ts = self.ds3231_timestamps[self.ds3231_ts_index];
            self.ds3231_ts_index = (self.ds3231_ts_index + 1) % self.ds3231_timestamps.len();
            self.ds3231_mock.set_timestamp(ts);
            if let Ok(dt) = self.rtc_sensor.read_datetime() {
                let mut s = String::new();
                let _ = write!(
                    s,
                    "{}-{:02}-{:02} {:02}:{:02}:{:02}",
                    dt.year(),
                    dt.month,
                    dt.day,
                    dt.hour,
                    dt.minute,
                    dt.second
                );
                self.last_rtc_str = s;
            }
        }

        // Poll VL53L0X ToF sensor every 4 ticks
        if tick == 1 || tick % 4 == 0 {
            if let Ok(reading) = self.tof_sensor.read_distance() {
                self.last_tof_mm = Some(reading.distance_mm);
            }
        }

        let servo_angle = distance_to_servo_angle(self.last_distance_mm.unwrap_or(180));
        self.servo
            .set_angle_degrees(servo_angle)
            .expect("servo angle should remain in range");

        let (left, right) = motor_commands_from_state(self.last_distance_mm, self.last_imu);
        self.motor_driver
            .apply_channels(left, right)
            .expect("motor commands should remain in range");

        let attached_devices = self
            .bus
            .attached_addresses()
            .into_iter()
            .map(|addr| format!("0x{addr:02X}"))
            .collect::<Vec<_>>();
        let recent_operations = self
            .bus
            .operations()
            .iter()
            .rev()
            .take(10)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(format_operation)
            .collect::<Vec<_>>();
        let app_frame = self
            .app
            .last_frame()
            .map(frame_to_lines)
            .unwrap_or_else(blank_lines);
        let physical_lcd_frame = frame_to_lines(self.lcd.frame());
        let climate = self.app.last_reading();
        let imu = self
            .last_imu
            .unwrap_or_else(|| hal_api::imu::ImuReading::new([0, 0, 0], [0, 0, 0], None));

        DeviceDashboardState {
            board_name: self.board.name().to_string(),
            mcu_name: self.board.mcu().to_string(),
            tick,
            climate: ClimatePanelState {
                temperature_c: climate.map(|value| value.temperature_centi_celsius as f32 / 100.0),
                humidity_percent: climate.map(|value| value.humidity_centi_percent as f32 / 100.0),
                pressure_pa: climate.and_then(|value| value.pressure_pascal),
                app_frame,
                physical_lcd_frame,
            },
            distance: DistancePanelState {
                distance_mm: self.last_distance_mm,
                sensor_name: "HC-SR04".to_string(),
            },
            imu: ImuPanelState {
                sensor_name: "MPU6050".to_string(),
                accel_mg: imu.accel_mg,
                gyro_mdps: imu.gyro_mdps,
                temperature_c: imu
                    .temperature_centi_celsius
                    .map(|value| value as f32 / 100.0),
            },
            servo: ServoPanelState {
                angle_degrees: self.servo.current_angle(),
            },
            motor_driver: MotorDriverPanelState {
                driver_name: "L298N dual H-bridge".to_string(),
                left: channel_state(self.motor_driver.channel_a().current_command()),
                right: channel_state(self.motor_driver.channel_b().current_command()),
            },
            wiring: WiringPanelState {
                sda_pin: self.board.sda_pin().to_string(),
                scl_pin: self.board.scl_pin().to_string(),
                power_pin: self.board.power_pin().to_string(),
                ground_pin: "GND".to_string(),
                diagram_lines: build_wiring_diagram(self.board, &attached_devices),
                attached_devices,
            },
            i2c: I2cPanelState {
                operation_count: self.bus.operation_count(),
                recent_operations,
            },
            light: LightPanelState {
                lux_x100: self.last_lux_x100,
                sensor_name: "BH1750".to_string(),
            },
            camera: CameraPanelState {
                width: self.camera.resolution().0,
                height: self.camera.resolution().1,
                sequence: self.last_camera_sequence,
                sensor_name: "ESP32-CAM".to_string(),
            },
            gas: GasPanelState {
                co2_ppm: self.last_gas.map(|g| g.co2_ppm),
                voc_ppb: self.last_gas.map(|g| g.voc_ppb),
                sensor_name: "SGP30".to_string(),
            },
            rtc: RtcPanelState {
                datetime_str: self.last_rtc_str.clone(),
                sensor_name: "DS3231".to_string(),
            },
            tof: TofPanelState {
                distance_mm: self.last_tof_mm,
                sensor_name: "VL53L0X".to_string(),
            },
        }
    }
}

fn blank_lines() -> [String; 2] {
    [
        String::from("                "),
        String::from("                "),
    ]
}

fn frame_to_lines(frame: hal_api::display::TextFrame16x2) -> [String; 2] {
    [line_to_string(&frame, 0), line_to_string(&frame, 1)]
}

fn line_to_string(frame: &hal_api::display::TextFrame16x2, row: usize) -> String {
    str::from_utf8(frame.line(row))
        .unwrap_or("????????????????")
        .to_string()
}

fn addr_to_device_name(addr: &str) -> &'static str {
    match addr {
        "0x23" => "BH1750",
        "0x27" => "LCD1602",
        "0x3C" => "SSD1306",
        "0x68" => "MPU6050",
        "0x77" => "BME280",
        _ => "unknown",
    }
}

fn build_wiring_diagram(board: BoardProfile, attached_devices: &[String]) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let sda_prefix = format!("{} SDA ", board.sda_pin());
    let cont = " ".repeat(sda_prefix.len());

    lines.push("── I2C Bus ──────────────────────────────────────".to_string());
    if attached_devices.is_empty() {
        lines.push(format!("{}---- (no devices)", sda_prefix));
    } else {
        for (i, addr) in attached_devices.iter().enumerate() {
            let name = addr_to_device_name(addr);
            if i == 0 {
                lines.push(format!("{}--+-- {} ({})", sda_prefix, name, addr));
            } else {
                lines.push(format!("{}  +-- {} ({})", cont, name, addr));
            }
        }
    }
    lines.push(format!("{} SCL ---- (shared bus)", board.scl_pin()));
    lines.push(format!("{} VCC ---- sensor power", board.power_pin()));
    lines.push("GND      ---- shared ground".to_string());
    lines.push(String::new());

    lines.push("── GPIO ─────────────────────────────────────────".to_string());
    lines.push(format!("{} TRIG --- HC-SR04 TRIG", board.trig_pin()));
    lines.push(format!("{} ECHO --- HC-SR04 ECHO", board.echo_pin()));
    lines.push(format!("{} PWM  --- Servo signal", board.servo_pwm_pin()));
    lines.push(String::new());

    lines.push("── Motor Driver (L298N-style) ───────────────────".to_string());
    lines.push(format!(
        "{} ENA  --- Motor-A enable (PWM)",
        board.motor_ena_pin()
    ));
    lines.push(format!(
        "{} IN1  --- Motor-A direction 1",
        board.motor_in1_pin()
    ));
    lines.push(format!(
        "{} IN2  --- Motor-A direction 2",
        board.motor_in2_pin()
    ));
    lines.push(format!(
        "{} ENB  --- Motor-B enable (PWM)",
        board.motor_enb_pin()
    ));
    lines.push(format!(
        "{} IN3  --- Motor-B direction 1",
        board.motor_in3_pin()
    ));
    lines.push(format!(
        "{} IN4  --- Motor-B direction 2",
        board.motor_in4_pin()
    ));

    lines
}

fn distance_to_servo_angle(distance_mm: u32) -> u16 {
    let clamped = distance_mm.clamp(80, 360) - 80;
    ((clamped * 180) / 280) as u16
}

fn motor_commands_from_state(
    distance_mm: Option<u32>,
    imu: Option<hal_api::imu::ImuReading>,
) -> (MotorCommand, MotorCommand) {
    let distance_mm = distance_mm.unwrap_or(180);
    let imu = imu.unwrap_or_else(|| hal_api::imu::ImuReading::new([0, 0, 0], [0, 0, 0], None));

    if distance_mm < 160 {
        return (
            MotorCommand::new(MotorDirection::Reverse, 30),
            MotorCommand::new(MotorDirection::Reverse, 30),
        );
    }

    let yaw = imu.gyro_mdps[2];
    if yaw > 40 {
        (
            MotorCommand::new(MotorDirection::Forward, 28),
            MotorCommand::new(MotorDirection::Forward, 46),
        )
    } else if yaw < -40 {
        (
            MotorCommand::new(MotorDirection::Forward, 46),
            MotorCommand::new(MotorDirection::Forward, 28),
        )
    } else {
        (
            MotorCommand::new(MotorDirection::Forward, 42),
            MotorCommand::new(MotorDirection::Forward, 42),
        )
    }
}

fn channel_state(command: MotorCommand) -> MotorChannelState {
    MotorChannelState {
        direction: match command.direction {
            MotorDirection::Forward => "forward",
            MotorDirection::Reverse => "reverse",
            MotorDirection::Brake => "brake",
            MotorDirection::Coast => "coast",
        }
        .to_string(),
        duty_percent: command.duty_percent,
    }
}

fn format_operation(operation: &VirtualI2cOperation) -> String {
    match operation {
        VirtualI2cOperation::Write { addr, bytes } => {
            format!("WRITE addr=0x{addr:02X} bytes={bytes:02X?}")
        }
        VirtualI2cOperation::Read { addr, len } => {
            format!("READ addr=0x{addr:02X} len={len}")
        }
        VirtualI2cOperation::WriteRead { addr, bytes, len } => {
            format!("WRITE_READ addr=0x{addr:02X} bytes={bytes:02X?} len={len}")
        }
    }
}

/// Extract the `"board"` string value from a minimal JSON object.
///
/// Handles `{"board":"arduino-nano"}` without pulling in a full JSON parser.
fn parse_board_from_json(json: &str) -> Option<&str> {
    let after_key = json.split("\"board\"").nth(1)?;
    let after_colon = after_key.split(':').nth(1)?.trim_start();
    let inner = after_colon.strip_prefix('"')?;
    let end = inner.find('"')?;
    Some(&inner[..end])
}

/// Extract `sensor_profile` field from a JSON body string.
///
/// Handles `{"sensor_profile":"climate"}` without a full JSON parser.
fn parse_sensor_profile_from_json(json: &str) -> Option<&str> {
    let after_key = json.split("\"sensor_profile\"").nth(1)?;
    let after_colon = after_key.split(':').nth(1)?.trim_start();
    let inner = after_colon.strip_prefix('"')?;
    let end = inner.find('"')?;
    Some(&inner[..end])
}

fn respond(stream: &mut TcpStream, status: &str, content_type: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());
}

/// Stream `cargo test --workspace` output as Server-Sent Events.
///
/// Blocks until the test process exits. Each stdout/stderr line is sent as
/// Returns available serial ports likely connected to an MCU.
fn list_serial_ports() -> Vec<String> {
    let Ok(dir) = std::fs::read_dir("/dev") else {
        return vec![];
    };
    let mut ports: Vec<String> = dir
        .filter_map(|e| e.ok())
        .map(|e| e.path().to_string_lossy().to_string())
        .filter(|name| {
            // macOS: cu.usbserial*, cu.SLAB*, cu.wchusbserial*, cu.usbmodem*
            // Linux: ttyUSB*, ttyACM*
            name.contains("/cu.usbserial")
                || name.contains("/cu.SLAB")
                || name.contains("/cu.wchusbserial")
                || name.contains("/cu.usbmodem")
                || name.contains("/ttyUSB")
                || name.contains("/ttyACM")
        })
        .collect();
    ports.sort();
    ports
}

/// Streams `espflash flash` output via Server-Sent Events.
///
/// Query params: `port=<serial-device>` (optional `bin=<path-to-elf>`)
/// Graceful degradation: if espflash is not installed, emits an instructional
/// error event so the UI can display installation guidance.
fn handle_flash_stream(stream: &mut TcpStream, query: &str) {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};
    use std::sync::mpsc;

    let header = "HTTP/1.1 200 OK\r\n\
        Content-Type: text/event-stream\r\n\
        Cache-Control: no-cache\r\n\
        Connection: keep-alive\r\n\
        Access-Control-Allow-Origin: *\r\n\
        \r\n";
    if stream.write_all(header.as_bytes()).is_err() {
        return;
    }

    // Parse query string: port=... (&bin=...)
    let port = query
        .split('&')
        .find_map(|kv| kv.strip_prefix("port="))
        .unwrap_or("")
        .to_string();
    let bin = query
        .split('&')
        .find_map(|kv| kv.strip_prefix("bin="))
        .unwrap_or("")
        .to_string();

    // Check espflash is available.
    if std::process::Command::new("espflash")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_err()
    {
        let _ = stream.write_all(
            b"data: [ERROR] espflash not found.\n\n\
              data: Install: cargo install espflash\n\n\
              data: Docs: https://github.com/esp-rs/espflash\n\n\
              data: [DONE] exit=1\n\n",
        );
        return;
    }

    if port.is_empty() {
        let _ = stream.write_all(
            b"data: [ERROR] No port specified. Use ?port=/dev/cu.usbserial-XXXX\n\ndata: [DONE] exit=1\n\n",
        );
        return;
    }

    let mut args = vec!["flash".to_string(), "--port".to_string(), port];
    if !bin.is_empty() {
        args.push(bin);
    }

    let mut child = match Command::new("espflash")
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            let _ = stream.write_all(
                format!("data: [ERROR] failed to spawn espflash: {e}\n\ndata: [DONE] exit=1\n\n")
                    .as_bytes(),
            );
            return;
        }
    };

    let (tx, rx) = mpsc::channel::<String>();
    let tx_out = tx.clone();
    let stdout = child.stdout.take().expect("stdout piped");
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if tx_out.send(line).is_err() {
                break;
            }
        }
    });
    let tx_err = tx.clone();
    let stderr = child.stderr.take().expect("stderr piped");
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            if tx_err.send(line).is_err() {
                break;
            }
        }
    });
    drop(tx);

    let started = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(120);

    for line in rx {
        if started.elapsed() > timeout {
            let _ = stream.write_all(
                b"data: [ERROR] timeout: espflash exceeded 2 minutes\n\ndata: [DONE] exit=1\n\n",
            );
            let _ = child.kill();
            return;
        }
        let msg = format!("data: {}\n\n", line.replace('\n', " "));
        if stream.write_all(msg.as_bytes()).is_err() {
            let _ = child.kill();
            return;
        }
    }

    let exit_code = child.wait().map(|s| s.code().unwrap_or(-1)).unwrap_or(-1);
    let _ = stream.write_all(format!("data: [DONE] exit={exit_code}\n\n").as_bytes());
}

/// `data: {line}\n\n`.  A final `data: [DONE] exit=N\n\n` closes the stream.
fn handle_test_stream(stream: &mut TcpStream) {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};
    use std::sync::mpsc;

    let header = "HTTP/1.1 200 OK\r\n\
        Content-Type: text/event-stream\r\n\
        Cache-Control: no-cache\r\n\
        Connection: keep-alive\r\n\
        Access-Control-Allow-Origin: *\r\n\
        \r\n";
    if stream.write_all(header.as_bytes()).is_err() {
        return;
    }

    let mut child = match Command::new("cargo")
        .args(["test", "--workspace", "--color=never"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            let _ = stream.write_all(
                format!("data: [ERROR] failed to spawn: {e}\n\ndata: [DONE] exit=1\n\n").as_bytes(),
            );
            return;
        }
    };

    let (tx, rx) = mpsc::channel::<String>();
    let tx_out = tx.clone();
    let stdout = child.stdout.take().expect("stdout piped");
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if tx_out.send(line).is_err() {
                break;
            }
        }
    });
    let tx_err = tx.clone();
    let stderr = child.stderr.take().expect("stderr piped");
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            if tx_err.send(line).is_err() {
                break;
            }
        }
    });
    drop(tx);

    let started = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(300);

    for line in rx {
        if started.elapsed() > timeout {
            let _ = stream.write_all(
                b"data: [ERROR] timeout: cargo test exceeded 5 minutes\n\ndata: [DONE] exit=1\n\n",
            );
            let _ = child.kill();
            return;
        }
        let msg = format!("data: {}\n\n", line.replace('\n', " "));
        if stream.write_all(msg.as_bytes()).is_err() {
            let _ = child.kill();
            return;
        }
    }

    let exit_code = child.wait().map(|s| s.code().unwrap_or(-1)).unwrap_or(-1);
    let _ = stream.write_all(format!("data: [DONE] exit={exit_code}\n\n").as_bytes());
}

fn main() {
    let mut args = env::args().skip(1);
    let first = args.next();
    let second = args.next();
    let board = BoardProfile::from_arg(first.as_deref());
    let port = second
        .as_deref()
        .and_then(|value| value.parse::<u16>().ok())
        .or_else(|| first.as_deref().and_then(|value| value.parse::<u16>().ok()))
        .unwrap_or(DEFAULT_PORT);

    let listener = TcpListener::bind(("127.0.0.1", port)).expect("server should bind");
    listener
        .set_nonblocking(true)
        .expect("non-blocking should be supported");

    let ctx = ServerContext::new(board);
    let (board_tx, board_rx) = mpsc::channel::<BoardProfile>();
    let mut rig = DeviceSimulationRig::new(board);
    let mut push_ticker: u32 = 0;

    println!("device dashboard server started");
    println!("open http://127.0.0.1:{port}");
    println!("board profile: {}", board.name());
    println!("WebSocket endpoint: ws://127.0.0.1:{port}/api/ws");

    loop {
        // Apply pending board change from a handler thread.
        if let Ok(new_board) = board_rx.try_recv() {
            rig = DeviceSimulationRig::new(new_board);
            *ctx.current_board.lock().unwrap() = new_board;
            println!("board changed to: {}", new_board.name());
        }

        // Tick the simulation.
        let state = rig.step();
        push_ticker = push_ticker.wrapping_add(1);

        // Push JSON to WebSocket clients every 10 ticks (~100 ms).
        if push_ticker % 10 == 0 {
            ctx.push_state(state_to_json(&state));
        }

        // Accept new TCP connections (non-blocking).
        loop {
            match listener.accept() {
                Ok((stream, _)) => {
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                    let ctx_clone = Arc::clone(&ctx);
                    let board_tx_clone = board_tx.clone();
                    thread::spawn(move || {
                        handle_connection(stream, ctx_clone, board_tx_clone);
                    });
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }

        thread::sleep(Duration::from_millis(10));
    }
}

fn handle_connection(
    mut stream: TcpStream,
    ctx: Arc<ServerContext>,
    board_tx: mpsc::Sender<BoardProfile>,
) {
    let mut request_buf = [0u8; 4096];
    let Ok(read_len) = stream.read(&mut request_buf) else {
        return;
    };
    let raw = &request_buf[..read_len];
    let request = String::from_utf8_lossy(raw);

    let first_line = request.lines().next().unwrap_or("GET / HTTP/1.1");
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or("GET");
    let path = parts.next().unwrap_or("/");
    let (path_only, query_str) = path.split_once('?').unwrap_or((path, ""));

    // Check for WebSocket upgrade before HTTP routing.
    let is_ws_upgrade = request.to_ascii_lowercase().contains("upgrade: websocket");
    if is_ws_upgrade && path_only == "/api/ws" {
        handle_websocket(stream, &request, ctx);
        return;
    }

    let body = request
        .find("\r\n\r\n")
        .map(|pos| &request[pos + 4..])
        .unwrap_or("");

    match (method, path_only) {
        (_, "/") => respond(
            &mut stream,
            "200 OK",
            "text/html; charset=utf-8",
            dashboard_html(),
        ),
        (_, "/api/state") => {
            let json = ctx.latest_json.lock().unwrap().clone();
            respond(
                &mut stream,
                "200 OK",
                "application/json; charset=utf-8",
                &json,
            );
        }
        ("POST", "/api/wiring") => {
            if let Some(board_name) = parse_board_from_json(body) {
                let new_board = BoardProfile::from_arg(Some(board_name));
                let _ = board_tx.send(new_board);
                // Give the main thread time to apply the change.
                thread::sleep(Duration::from_millis(50));
            }
            if let Some(profile_slug) = parse_sensor_profile_from_json(body) {
                if let Some(profile) = SensorProfile::from_slug(profile_slug) {
                    *ctx.current_sensor_profile.lock().unwrap() = profile;
                }
            }
            let board = *ctx.current_board.lock().unwrap();
            let sensor_profile = *ctx.current_sensor_profile.lock().unwrap();
            let payload = WiringConfig::from_board_with_sensors(board, sensor_profile).to_json();
            respond(
                &mut stream,
                "200 OK",
                "application/json; charset=utf-8",
                &payload,
            );
        }
        (_, "/api/wiring/profiles") => {
            let entries: Vec<String> = SensorProfile::all()
                .iter()
                .map(|(slug, name)| format!(r#"{{"slug":"{slug}","name":"{name}"}}"#))
                .collect();
            let payload = format!(r#"{{"profiles":[{}]}}"#, entries.join(","));
            respond(
                &mut stream,
                "200 OK",
                "application/json; charset=utf-8",
                &payload,
            );
        }
        (_, "/api/wiring") => {
            let board = *ctx.current_board.lock().unwrap();
            let sensor_profile = *ctx.current_sensor_profile.lock().unwrap();
            let payload = WiringConfig::from_board_with_sensors(board, sensor_profile).to_json();
            respond(
                &mut stream,
                "200 OK",
                "application/json; charset=utf-8",
                &payload,
            );
        }
        (_, "/api/wiring/svg") => {
            let board = *ctx.current_board.lock().unwrap();
            let sensor_profile = *ctx.current_sensor_profile.lock().unwrap();
            let cfg = WiringConfig::from_board_with_sensors(board, sensor_profile);
            let svg = wiring_svg(&cfg);
            respond(&mut stream, "200 OK", "image/svg+xml; charset=utf-8", &svg);
        }
        ("POST", "/api/wiring/editor") => {
            *ctx.editor_json.lock().unwrap() = body.to_string();
            respond(
                &mut stream,
                "200 OK",
                "application/json; charset=utf-8",
                r#"{"ok":true}"#,
            );
        }
        (_, "/api/wiring/editor") => {
            let json = ctx.editor_json.lock().unwrap().clone();
            respond(
                &mut stream,
                "200 OK",
                "application/json; charset=utf-8",
                &json,
            );
        }
        (_, "/api/test/stream") => {
            handle_test_stream(&mut stream);
        }
        (_, "/api/flash/devices") => {
            let ports = list_serial_ports();
            let json = format!(
                "[{}]",
                ports
                    .iter()
                    .map(|p| format!("\"{}\"", p))
                    .collect::<Vec<_>>()
                    .join(",")
            );
            respond(
                &mut stream,
                "200 OK",
                "application/json; charset=utf-8",
                &json,
            );
        }
        (_, "/api/flash/stream") => {
            handle_flash_stream(&mut stream, query_str);
        }
        _ => respond(
            &mut stream,
            "404 Not Found",
            "text/plain; charset=utf-8",
            "not found",
        ),
    }
}

/// Upgrade the TCP stream to WebSocket, then stream JSON state updates to the client.
///
/// Implements a minimal RFC 6455 WebSocket server-side handshake and text-frame sender
/// using only SHA-1 (via `sha1_smol`) and Base64 (`base64`) — no full WS library needed.
fn handle_websocket(mut stream: TcpStream, request_headers: &str, ctx: Arc<ServerContext>) {
    // ── Handshake ─────────────────────────────────────────────────────────────
    let ws_key = request_headers
        .lines()
        .find_map(|line| {
            let lower = line.to_ascii_lowercase();
            if lower.starts_with("sec-websocket-key:") {
                Some(line[line.find(':').unwrap() + 1..].trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_default();

    const GUID: &str = "258EAFA5-E914-4789-0000-000000000000";
    let accept_input = format!("{ws_key}{GUID}");
    let digest = sha1_smol::Sha1::from(accept_input.as_bytes())
        .digest()
        .bytes();
    use base64::Engine as _;
    let accept_value = base64::engine::general_purpose::STANDARD.encode(digest);

    let response = format!(
        "HTTP/1.1 101 Switching Protocols\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Accept: {accept_value}\r\n\
         \r\n"
    );
    if stream.write_all(response.as_bytes()).is_err() {
        return;
    }

    // ── Register a push channel ────────────────────────────────────────────────
    let initial = ctx.latest_json.lock().unwrap().clone();
    let (tx, rx) = mpsc::sync_channel::<String>(32);
    ctx.ws_clients.lock().unwrap().push(tx);

    // Send the current state immediately so the client doesn't have to wait.
    if ws_text_send(&mut stream, &initial).is_err() {
        return;
    }

    // Forward state updates until the channel or stream is closed.
    for json in rx {
        if ws_text_send(&mut stream, &json).is_err() {
            break;
        }
    }
}

/// Send a single unsegmented text frame (RFC 6455 §5.6, no masking on server side).
fn ws_text_send(stream: &mut TcpStream, payload: &str) -> io::Result<()> {
    let data = payload.as_bytes();
    let len = data.len();

    let mut header = Vec::with_capacity(10);
    header.push(0x81u8); // FIN=1, opcode=0x1 (text)
    if len < 126 {
        header.push(len as u8);
    } else if len < 65536 {
        header.push(0x7E);
        header.push((len >> 8) as u8);
        header.push((len & 0xFF) as u8);
    } else {
        header.push(0x7F);
        for shift in (0..8).rev() {
            header.push(((len >> (shift * 8)) & 0xFF) as u8);
        }
    }
    stream.write_all(&header)?;
    stream.write_all(data)?;
    stream.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_to_servo_angle_at_minimum() {
        assert_eq!(distance_to_servo_angle(80), 0);
    }

    #[test]
    fn distance_to_servo_angle_at_maximum() {
        assert_eq!(distance_to_servo_angle(360), 180);
    }

    #[test]
    fn distance_to_servo_angle_at_midpoint() {
        // 220mm → clamped = 220 - 80 = 140 → (140 * 180) / 280 = 90
        let angle = distance_to_servo_angle(220);
        assert!(angle > 60 && angle < 120, "expected ~90, got {angle}");
    }

    #[test]
    fn motor_commands_from_state_reverses_when_obstacle_close() {
        // distance < 160 → both channels Reverse
        let (left, right) = motor_commands_from_state(Some(100), None);
        assert_eq!(left.direction, MotorDirection::Reverse);
        assert_eq!(right.direction, MotorDirection::Reverse);
    }

    #[test]
    fn motor_commands_from_state_drives_forward_when_clear() {
        // distance = 200 >= 160, no IMU tilt → both channels Forward straight
        let (left, right) = motor_commands_from_state(Some(200), None);
        assert_eq!(left.direction, MotorDirection::Forward);
        assert_eq!(right.direction, MotorDirection::Forward);
        assert_eq!(left.duty_percent, right.duty_percent);
    }

    #[test]
    fn parse_board_from_json_extracts_board_name() {
        assert_eq!(
            parse_board_from_json(r#"{"board":"arduino-nano"}"#),
            Some("arduino-nano")
        );
    }

    #[test]
    fn parse_board_from_json_handles_esp32_value() {
        assert_eq!(
            parse_board_from_json(r#"{"board":"original-esp32"}"#),
            Some("original-esp32")
        );
    }

    #[test]
    fn parse_board_from_json_returns_none_for_missing_key() {
        assert_eq!(parse_board_from_json(r#"{"other":"value"}"#), None);
    }

    #[test]
    fn parse_board_from_json_returns_none_for_empty_body() {
        assert_eq!(parse_board_from_json(""), None);
    }

    #[test]
    fn parse_sensor_profile_from_json_extracts_profile() {
        assert_eq!(
            parse_sensor_profile_from_json(r#"{"sensor_profile":"climate"}"#),
            Some("climate")
        );
    }

    #[test]
    fn parse_sensor_profile_from_json_handles_combined_body() {
        assert_eq!(
            parse_sensor_profile_from_json(r#"{"board":"esp32","sensor_profile":"robot"}"#),
            Some("robot")
        );
    }

    #[test]
    fn parse_sensor_profile_from_json_returns_none_for_missing_key() {
        assert_eq!(parse_sensor_profile_from_json(r#"{"board":"esp32"}"#), None);
    }

    #[test]
    fn parse_sensor_profile_from_json_returns_none_for_empty_body() {
        assert_eq!(parse_sensor_profile_from_json(""), None);
    }
}
