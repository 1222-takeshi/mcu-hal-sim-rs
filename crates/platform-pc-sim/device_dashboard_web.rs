use core::str;
use std::env;
use std::fmt::Write as FmtWrite;
use std::io::{self, Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use core_app::climate_display::{frame_from_reading, ClimateDisplayApp, ClimateDisplayConfig};
use embedded_hal::delay::DelayNs;
use hal_api::actuator::{DualMotorDriver, MotorCommand, MotorDirection, ServoMotor};
use hal_api::camera::CameraCapture;
use hal_api::distance::DistanceSensor;
use hal_api::gas::GasSensor;
use hal_api::imu::ImuSensor;
use hal_api::light::LightSensor;
use hal_api::rtc::RtcSensor;
use hal_api::sensor::EnvSensor;
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
use platform_pc_sim::wiring_config::{ConnectionType, DeviceKind, SensorProfile, WiringConfig};
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
const DS3231_SIM_ADDRESS: u8 = DS3231_ADDRESS + 1;

/// Combined board + sensor profile state read/written as a unit.
#[derive(Clone)]
struct WiringState {
    board: BoardProfile,
    sensor_profile: SensorProfile,
    selected_devices: Vec<DeviceKind>,
    show_bus_labels: bool,
}

fn dashboard_wiring_config(
    board: BoardProfile,
    sensor_profile: SensorProfile,
    selected_devices: &[DeviceKind],
    show_bus_labels: bool,
) -> WiringConfig {
    WiringConfig::from_board_with_selected_devices(board, sensor_profile, selected_devices)
        .with_bus_labels(show_bus_labels)
}

/// Shared server state passed to every connection-handler thread.
struct ServerContext {
    latest_json: Mutex<String>,
    sse_clients: Mutex<Vec<mpsc::SyncSender<String>>>,
    current_board: Mutex<BoardProfile>,
    /// Kept for future use; wiring API reads now go through `wiring_state`.
    #[allow(dead_code)]
    current_sensor_profile: Mutex<SensorProfile>,
    /// Atomic board + sensor profile state for wiring API reads.
    wiring_state: Mutex<WiringState>,
    /// Last wiring-editor JSON submitted via POST /api/wiring/editor.
    editor_json: Mutex<String>,
}

impl ServerContext {
    fn new(board: BoardProfile) -> Arc<Self> {
        Arc::new(Self {
            latest_json: Mutex::new("{}".into()),
            sse_clients: Mutex::new(vec![]),
            current_board: Mutex::new(board),
            current_sensor_profile: Mutex::new(SensorProfile::Full),
            wiring_state: Mutex::new(WiringState {
                board,
                sensor_profile: SensorProfile::Full,
                selected_devices: SensorProfile::Full.device_kinds(),
                show_bus_labels: false,
            }),
            editor_json: Mutex::new("{}".into()),
        })
    }

    fn push_state(&self, json: String) {
        *self.latest_json.lock().unwrap() = json.clone();
        self.sse_clients
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
    climate_sensor: Bme280Sensor<VirtualI2cBus>,
    app: ClimateDisplayApp<Bme280Sensor<VirtualI2cBus>, Lcd1602Display<VirtualI2cBus, NoopDelay>>,
    bme280_samples: Vec<[u8; 8]>,
    bme280_sample_index: usize,
    distance_sensor: HcSr04Sensor<MockHcSr04Device>,
    imu_sensor: Mpu6050Sensor<VirtualI2cBus>,
    imu_frames: Vec<[u8; 14]>,
    imu_frame_index: usize,
    servo: ServoRig,
    motor_driver: MotorDriverRig,
    tick: u32,
    last_distance_mm: Option<u32>,
    last_imu: Option<hal_api::imu::ImuReading>,
    light_sensor: Bh1750Sensor<VirtualI2cBus>,
    bh1750_mock: platform_pc_sim::bh1750_mock::MockBh1750Device,
    camera: MockCamera,
    last_lux_x100: u32,
    last_camera_sequence: u32,
    // New sensors: SGP30, DS3231, VL53L0X
    ds3231_mock: MockDs3231Device,
    ds3231_timestamps: Vec<platform_pc_sim::ds3231_mock::MockRtcTimestamp>,
    ds3231_ts_index: usize,
    rtc_sensor: Ds3231Sensor<VirtualI2cBus>,
    sgp30_mock: MockSgp30Device,
    sgp30_sensor: Sgp30Sensor<VirtualI2cBus>,
    vl53l0x_mock: MockVl53l0xDevice,
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
        let bh1750_mock = platform_pc_sim::bh1750_mock::MockBh1750Device::looping(vec![
            8_500, 12_000, 15_300, 9_800, 7_200,
        ]);
        // DS3231 shares 0x68 with MPU6050 in real hardware, so the simulator
        // attaches it internally at 0x69 to avoid the collision. Dashboard
        // surfaces translate it back to the logical hardware address 0x68.
        let ds3231_mock = MockDs3231Device::new();
        let sgp30_mock = MockSgp30Device::new();
        let vl53l0x_mock = MockVl53l0xDevice::new();

        bus.attach_device(BME280_ADDRESS_PRIMARY, bme280.clone());
        bus.attach_device(LCD1602_ADDRESS_PRIMARY, lcd.clone());
        bus.attach_device(MPU6050_ADDRESS_PRIMARY, mpu6050.clone());
        bus.attach_device(BH1750_ADDRESS_LOW, bh1750_mock.clone());
        bus.attach_device(DS3231_SIM_ADDRESS, ds3231_mock.clone());
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
        let climate_sensor = Bme280Sensor::new(bus.clone());
        let distance_sensor = HcSr04Sensor::new(MockHcSr04Device::looping(demo_echo_pulses_us()));
        let imu_sensor = Mpu6050Sensor::new(bus.clone());
        let light_sensor = Bh1750Sensor::new(bus.clone(), BH1750_ADDRESS_LOW)
            .expect("BH1750 mock device should initialise");
        let camera = MockCamera::qvga_jpeg();
        let rtc_sensor = Ds3231Sensor::new(bus.clone(), DS3231_SIM_ADDRESS);
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
            climate_sensor,
            app,
            bme280_samples: demo_raw_samples(),
            bme280_sample_index: 0,
            distance_sensor,
            imu_sensor,
            imu_frames: demo_raw_frames(),
            imu_frame_index: 0,
            servo,
            motor_driver,
            tick: 0,
            last_distance_mm: None,
            last_imu: None,
            light_sensor,
            bh1750_mock,
            camera,
            last_lux_x100: 0,
            last_camera_sequence: 0,
            ds3231_mock,
            ds3231_timestamps: demo_timestamps(),
            ds3231_ts_index: 0,
            rtc_sensor,
            sgp30_mock,
            sgp30_sensor,
            vl53l0x_mock,
            tof_sensor,
            last_gas: None,
            last_rtc_str: String::new(),
            last_tof_mm: None,
        }
    }

    fn sync_selected_devices(&mut self, selected_devices: &[DeviceKind]) {
        if selected_devices.contains(&DeviceKind::Bme280) {
            self.bus
                .attach_device(BME280_ADDRESS_PRIMARY, self.bme280.clone());
        } else {
            self.bus.detach_device(BME280_ADDRESS_PRIMARY);
        }

        if selected_devices.contains(&DeviceKind::Lcd1602) {
            self.bus
                .attach_device(LCD1602_ADDRESS_PRIMARY, self.lcd.clone());
        } else {
            self.bus.detach_device(LCD1602_ADDRESS_PRIMARY);
        }

        if selected_devices.contains(&DeviceKind::Mpu6050) {
            self.bus
                .attach_device(MPU6050_ADDRESS_PRIMARY, self.mpu6050.clone());
        } else {
            self.bus.detach_device(MPU6050_ADDRESS_PRIMARY);
        }

        if selected_devices.contains(&DeviceKind::Bh1750) {
            self.bus
                .attach_device(BH1750_ADDRESS_LOW, self.bh1750_mock.clone());
        } else {
            self.bus.detach_device(BH1750_ADDRESS_LOW);
        }

        if selected_devices.contains(&DeviceKind::Ds3231) {
            self.bus
                .attach_device(DS3231_SIM_ADDRESS, self.ds3231_mock.clone());
        } else {
            self.bus.detach_device(DS3231_SIM_ADDRESS);
        }

        if selected_devices.contains(&DeviceKind::Sgp30) {
            self.bus
                .attach_device(SGP30_ADDRESS, self.sgp30_mock.clone());
        } else {
            self.bus.detach_device(SGP30_ADDRESS);
        }

        if selected_devices.contains(&DeviceKind::Vl53l0x) {
            self.bus
                .attach_device(VL53L0X_ADDRESS, self.vl53l0x_mock.clone());
        } else {
            self.bus.detach_device(VL53L0X_ADDRESS);
        }
    }

    fn step(&mut self, wiring_state: &WiringState) -> DeviceDashboardState {
        self.tick = self.tick.wrapping_add(1);
        let tick = self.tick;
        self.sync_selected_devices(&wiring_state.selected_devices);
        self.bus.clear_operations();

        let is_enabled = |kind: DeviceKind| wiring_state.selected_devices.contains(&kind);
        let bme280_enabled = is_enabled(DeviceKind::Bme280);
        let lcd_enabled = is_enabled(DeviceKind::Lcd1602);

        if !is_enabled(DeviceKind::HcSr04) {
            self.last_distance_mm = None;
        }
        if !is_enabled(DeviceKind::Mpu6050) {
            self.last_imu = None;
        }
        if !is_enabled(DeviceKind::Bh1750) {
            self.last_lux_x100 = 0;
        }
        if !is_enabled(DeviceKind::Esp32Cam) {
            self.last_camera_sequence = 0;
        }
        if !is_enabled(DeviceKind::Sgp30) {
            self.last_gas = None;
        }
        if !is_enabled(DeviceKind::Ds3231) {
            self.last_rtc_str.clear();
        }
        if !is_enabled(DeviceKind::Vl53l0x) {
            self.last_tof_mm = None;
        }

        if bme280_enabled {
            self.bme280
                .set_raw_sample(self.bme280_samples[self.bme280_sample_index]);
            self.bme280_sample_index = (self.bme280_sample_index + 1) % self.bme280_samples.len();
        }

        if is_enabled(DeviceKind::Mpu6050) {
            self.mpu6050
                .set_raw_frame(self.imu_frames[self.imu_frame_index]);
            self.imu_frame_index = (self.imu_frame_index + 1) % self.imu_frames.len();
        }

        if bme280_enabled && lcd_enabled {
            self.app
                .tick()
                .expect("dashboard climate app should keep running");
        }
        if is_enabled(DeviceKind::HcSr04) && (tick == 1 || tick % 2 == 0) {
            self.last_distance_mm = Some(
                self.distance_sensor
                    .read_distance()
                    .expect("distance driver should read from host-side pulse device")
                    .distance_mm,
            );
        }

        if is_enabled(DeviceKind::Mpu6050) && (tick == 1 || tick % 3 == 0) {
            self.last_imu = Some(
                self.imu_sensor
                    .read_imu()
                    .expect("imu driver should read from host-side mock device"),
            );
        }

        if is_enabled(DeviceKind::Bh1750) && (tick == 1 || tick % 5 == 0) {
            if let Ok(reading) = self.light_sensor.read_lux() {
                self.last_lux_x100 = reading.lux_x100;
            }
        }

        if is_enabled(DeviceKind::Esp32Cam) && (tick == 1 || tick % 7 == 0) {
            if let Ok(frame) = self.camera.capture_frame() {
                self.last_camera_sequence = frame.sequence;
            }
        }

        // Poll SGP30 gas sensor every 11 ticks
        if is_enabled(DeviceKind::Sgp30) && (tick == 1 || tick % 11 == 0) {
            if let Ok(reading) = self.sgp30_sensor.read_gas() {
                self.last_gas = Some(reading);
            }
        }

        // Poll DS3231 RTC every tick so /api/state recent_operations always reflects the
        // currently selected simulator address.
        if is_enabled(DeviceKind::Ds3231) {
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
        if is_enabled(DeviceKind::Vl53l0x) && (tick == 1 || tick % 4 == 0) {
            if let Ok(reading) = self.tof_sensor.read_distance() {
                self.last_tof_mm = Some(reading.distance_mm);
            }
        }

        if is_enabled(DeviceKind::Servo) {
            let servo_angle = if is_enabled(DeviceKind::HcSr04) {
                distance_to_servo_angle(self.last_distance_mm.unwrap_or(180))
            } else {
                0
            };
            self.servo
                .set_angle_degrees(servo_angle)
                .expect("servo angle should remain in range");
        } else {
            self.servo
                .set_angle_degrees(0)
                .expect("disabled servo should reset to zero angle");
        }

        if is_enabled(DeviceKind::L298n) {
            let (left, right) = if is_enabled(DeviceKind::HcSr04) && is_enabled(DeviceKind::Mpu6050)
            {
                motor_commands_from_state(self.last_distance_mm, self.last_imu)
            } else {
                (
                    MotorCommand::new(MotorDirection::Coast, 0),
                    MotorCommand::new(MotorDirection::Coast, 0),
                )
            };
            self.motor_driver
                .apply_channels(left, right)
                .expect("motor commands should remain in range");
        } else {
            self.motor_driver
                .apply_channels(disabled_motor_command(), disabled_motor_command())
                .expect("disabled motor driver should reset to coast");
        }

        let wiring_config = dashboard_wiring_config(
            wiring_state.board,
            wiring_state.sensor_profile,
            &wiring_state.selected_devices,
            wiring_state.show_bus_labels,
        );
        let attached_devices = wiring_config
            .devices
            .iter()
            .map(|device| device.label.clone())
            .collect::<Vec<_>>();
        let attached_addresses = self
            .bus
            .attached_addresses()
            .into_iter()
            .map(display_i2c_addr)
            .collect::<Vec<_>>();
        let recent_operations = self
            .bus
            .operations()
            .iter()
            .rev()
            .filter(|operation| attached_addresses.contains(&operation_addr(operation)))
            .take(10)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(format_operation)
            .collect::<Vec<_>>();
        let physical_lcd_frame = frame_to_lines(self.lcd.frame());
        let climate = if bme280_enabled {
            if lcd_enabled {
                self.app.last_reading()
            } else {
                self.climate_sensor.read().ok()
            }
        } else {
            None
        };
        let app_frame = if lcd_enabled {
            self.app
                .last_frame()
                .map(frame_to_lines)
                .unwrap_or_else(blank_lines)
        } else {
            climate
                .and_then(|reading| frame_from_reading(reading).ok())
                .map(frame_to_lines)
                .unwrap_or_else(blank_lines)
        };
        let imu = self
            .last_imu
            .unwrap_or_else(|| hal_api::imu::ImuReading::new([0, 0, 0], [0, 0, 0], None));
        let lcd_frame = if bme280_enabled && lcd_enabled {
            physical_lcd_frame
        } else {
            blank_lines()
        };

        DeviceDashboardState {
            board_name: self.board.name().to_string(),
            mcu_name: self.board.mcu().to_string(),
            tick,
            climate: ClimatePanelState {
                temperature_c: if bme280_enabled {
                    climate.map(|value| value.temperature_centi_celsius as f32 / 100.0)
                } else {
                    None
                },
                humidity_percent: if bme280_enabled {
                    climate.map(|value| value.humidity_centi_percent as f32 / 100.0)
                } else {
                    None
                },
                pressure_pa: if bme280_enabled {
                    climate.and_then(|value| value.pressure_pascal)
                } else {
                    None
                },
                app_frame: if lcd_enabled {
                    app_frame
                } else {
                    blank_lines()
                },
                physical_lcd_frame: lcd_frame,
            },
            distance: DistancePanelState {
                distance_mm: if is_enabled(DeviceKind::HcSr04) {
                    self.last_distance_mm
                } else {
                    None
                },
                sensor_name: "HC-SR04".to_string(),
            },
            imu: ImuPanelState {
                sensor_name: "MPU6050".to_string(),
                accel_mg: if is_enabled(DeviceKind::Mpu6050) {
                    imu.accel_mg
                } else {
                    [0, 0, 0]
                },
                gyro_mdps: if is_enabled(DeviceKind::Mpu6050) {
                    imu.gyro_mdps
                } else {
                    [0, 0, 0]
                },
                temperature_c: if is_enabled(DeviceKind::Mpu6050) {
                    imu.temperature_centi_celsius
                        .map(|value| value as f32 / 100.0)
                } else {
                    None
                },
            },
            servo: ServoPanelState {
                angle_degrees: if is_enabled(DeviceKind::Servo) {
                    self.servo.current_angle()
                } else {
                    0
                },
            },
            motor_driver: MotorDriverPanelState {
                driver_name: "L298N dual H-bridge".to_string(),
                left: if is_enabled(DeviceKind::L298n) {
                    channel_state(self.motor_driver.channel_a().current_command())
                } else {
                    channel_state(MotorCommand::new(MotorDirection::Coast, 0))
                },
                right: if is_enabled(DeviceKind::L298n) {
                    channel_state(self.motor_driver.channel_b().current_command())
                } else {
                    channel_state(MotorCommand::new(MotorDirection::Coast, 0))
                },
            },
            wiring: WiringPanelState {
                sda_pin: wiring_config.sda_pin.clone(),
                scl_pin: wiring_config.scl_pin.clone(),
                power_pin: wiring_config.power_pin.clone(),
                ground_pin: wiring_config.ground_pin.clone(),
                diagram_lines: build_wiring_diagram(&wiring_config),
                attached_devices,
                selected_devices: wiring_state
                    .selected_devices
                    .iter()
                    .map(|kind| kind.slug().to_string())
                    .collect(),
                show_bus_labels: wiring_state.show_bus_labels,
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

fn build_wiring_diagram(config: &WiringConfig) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let sda_prefix = format!("{} SDA ", config.sda_pin);
    let cont = " ".repeat(sda_prefix.len());
    let i2c_devices: Vec<_> = config
        .devices
        .iter()
        .filter(|device| device.kind.connection_type() == ConnectionType::I2c)
        .collect();
    let gpio_devices: Vec<_> = config
        .devices
        .iter()
        .filter(|device| device.kind.connection_type() == ConnectionType::Gpio)
        .collect();
    let pwm_devices: Vec<_> = config
        .devices
        .iter()
        .filter(|device| device.kind.connection_type() == ConnectionType::Pwm)
        .collect();

    lines.push("── I2C Bus ──────────────────────────────────────".to_string());
    if i2c_devices.is_empty() {
        lines.push(format!("{}---- (no devices)", sda_prefix));
    } else {
        for (i, device) in i2c_devices.iter().enumerate() {
            if i == 0 {
                lines.push(format!("{}--+-- {}", sda_prefix, device.label));
            } else {
                lines.push(format!("{}  +-- {}", cont, device.label));
            }
        }
    }
    lines.push(format!("{} SCL ---- (shared bus)", config.scl_pin));
    lines.push(format!("{} VCC ---- sensor power", config.power_pin));
    lines.push("GND      ---- shared ground".to_string());
    lines.push(String::new());

    lines.push("── GPIO ─────────────────────────────────────────".to_string());
    if gpio_devices.is_empty() {
        lines.push("(none selected)".to_string());
    } else {
        for device in gpio_devices {
            match device.kind {
                DeviceKind::HcSr04 => {
                    lines.push(format!("{} TRIG --- HC-SR04 TRIG", config.trig_pin));
                    lines.push(format!("{} ECHO --- HC-SR04 ECHO", config.echo_pin));
                }
                DeviceKind::Esp32Cam => {
                    lines.push(format!(
                        "{} GPIO --- ESP32-CAM boot/control",
                        config.cam_pin
                    ));
                }
                _ => {}
            }
        }
    }
    lines.push(String::new());

    lines.push("── PWM / Motor ──────────────────────────────────".to_string());
    if pwm_devices.is_empty() {
        lines.push("(none selected)".to_string());
    } else {
        for device in pwm_devices {
            match device.kind {
                DeviceKind::Servo => {
                    lines.push(format!("{} PWM  --- Servo signal", config.servo_pin));
                }
                DeviceKind::L298n => {
                    lines.push(format!(
                        "{} ENA  --- Motor-A enable (PWM)",
                        config.motor_pin
                    ));
                    lines.push(format!(
                        "{} IN1  --- Motor-A direction 1",
                        config.board.motor_in1_pin()
                    ));
                    lines.push(format!(
                        "{} IN2  --- Motor-A direction 2",
                        config.board.motor_in2_pin()
                    ));
                    lines.push(format!(
                        "{} ENB  --- Motor-B enable (PWM)",
                        config.board.motor_enb_pin()
                    ));
                    lines.push(format!(
                        "{} IN3  --- Motor-B direction 1",
                        config.board.motor_in3_pin()
                    ));
                    lines.push(format!(
                        "{} IN4  --- Motor-B direction 2",
                        config.board.motor_in4_pin()
                    ));
                }
                _ => {}
            }
        }
    }

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

fn disabled_motor_command() -> MotorCommand {
    MotorCommand::new(MotorDirection::Coast, 0)
}

fn display_i2c_addr(addr: u8) -> u8 {
    if addr == DS3231_SIM_ADDRESS {
        DS3231_ADDRESS
    } else {
        addr
    }
}

fn operation_addr(operation: &VirtualI2cOperation) -> u8 {
    match operation {
        VirtualI2cOperation::Write { addr, .. }
        | VirtualI2cOperation::Read { addr, .. }
        | VirtualI2cOperation::WriteRead { addr, .. } => display_i2c_addr(*addr),
    }
}

fn format_operation(operation: &VirtualI2cOperation) -> String {
    match operation {
        VirtualI2cOperation::Write { addr, bytes } => {
            let addr = display_i2c_addr(*addr);
            format!("WRITE addr=0x{addr:02X} bytes={bytes:02X?}")
        }
        VirtualI2cOperation::Read { addr, len } => {
            let addr = display_i2c_addr(*addr);
            format!("READ addr=0x{addr:02X} len={len}")
        }
        VirtualI2cOperation::WriteRead { addr, bytes, len } => {
            let addr = display_i2c_addr(*addr);
            format!("WRITE_READ addr=0x{addr:02X} bytes={bytes:02X?} len={len}")
        }
    }
}

/// Extract a string field value from a minimal JSON body.
///
/// Handles `{"key":"value"}` without a full JSON parser.
fn parse_json_string_field<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let key_literal = format!("\"{key}\"");
    let after_key = json.split(key_literal.as_str()).nth(1)?;
    let after_colon = after_key.split(':').nth(1)?.trim_start();
    let inner = after_colon.strip_prefix('"')?;
    let end = inner.find('"')?;
    Some(&inner[..end])
}

/// Extract the `"board"` string value from a minimal JSON object.
///
/// Handles `{"board":"arduino-nano"}` without pulling in a full JSON parser.
fn parse_board_from_json(json: &str) -> Option<&str> {
    parse_json_string_field(json, "board")
}

/// Extract `sensor_profile` field from a JSON body string.
///
/// Handles `{"sensor_profile":"climate"}` without a full JSON parser.
fn parse_sensor_profile_from_json(json: &str) -> Option<&str> {
    parse_json_string_field(json, "sensor_profile")
}

fn parse_json_string_array_field(json: &str, key: &str) -> Option<Vec<String>> {
    let key_literal = format!("\"{key}\"");
    let after_key = json.split(key_literal.as_str()).nth(1)?;
    let after_colon = after_key.split(':').nth(1)?.trim_start();
    let inner = after_colon.strip_prefix('[')?;
    let end = inner.find(']')?;
    let values = inner[..end].trim();
    if values.is_empty() {
        return Some(vec![]);
    }

    Some(
        values
            .split(',')
            .filter_map(|entry| {
                let trimmed = entry.trim();
                let without_prefix = trimmed.strip_prefix('"')?;
                let end = without_prefix.find('"')?;
                Some(without_prefix[..end].to_string())
            })
            .collect(),
    )
}

fn parse_json_bool_field(json: &str, key: &str) -> Option<bool> {
    let key_literal = format!("\"{key}\"");
    let after_key = json.split(key_literal.as_str()).nth(1)?;
    let after_colon = after_key.split(':').nth(1)?.trim_start();
    if after_colon.starts_with("true") {
        Some(true)
    } else if after_colon.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

#[cfg(test)]
fn parse_selected_devices_from_json(json: &str) -> Vec<String> {
    parse_json_string_array_field(json, "selected_devices").unwrap_or_default()
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
    println!("SSE endpoint: http://127.0.0.1:{port}/api/events");

    loop {
        // Apply pending board change from a handler thread.
        if let Ok(new_board) = board_rx.try_recv() {
            rig = DeviceSimulationRig::new(new_board);
            *ctx.current_board.lock().unwrap() = new_board;
            println!("board changed to: {}", new_board.name());
        }

        // Tick the simulation.
        let wiring_state = ctx.wiring_state.lock().unwrap().clone();
        let state = rig.step(&wiring_state);
        push_ticker = push_ticker.wrapping_add(1);

        // Push JSON to SSE clients every 10 ticks (~100 ms).
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
        (_, "/api/events") => {
            handle_sse_events(&mut stream, ctx);
        }
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
            // Update wiring_state atomically as a single unit.
            let wiring = {
                let mut ws = ctx.wiring_state.lock().unwrap();
                if let Some(board_name) = parse_board_from_json(body) {
                    ws.board = BoardProfile::from_arg(Some(board_name));
                }
                if let Some(profile_slug) = parse_sensor_profile_from_json(body) {
                    if let Some(profile) = SensorProfile::from_slug(profile_slug) {
                        ws.sensor_profile = profile;
                        ws.selected_devices = profile.device_kinds();
                    }
                }
                if let Some(selected_devices) =
                    parse_json_string_array_field(body, "selected_devices")
                {
                    ws.selected_devices = selected_devices
                        .into_iter()
                        .filter_map(|slug| DeviceKind::from_slug(&slug))
                        .collect();
                }
                if let Some(show_bus_labels) = parse_json_bool_field(body, "show_bus_labels") {
                    ws.show_bus_labels = show_bus_labels;
                }
                ws.clone()
            };
            let payload = dashboard_wiring_config(
                wiring.board,
                wiring.sensor_profile,
                &wiring.selected_devices,
                wiring.show_bus_labels,
            )
            .to_json();
            respond(
                &mut stream,
                "200 OK",
                "application/json; charset=utf-8",
                &payload,
            );
        }
        (_, "/api/wiring/profiles") => {
            let entries: Vec<String> = SensorProfile::all_variants()
                .iter()
                .map(|p| {
                    let devices = p
                        .device_kinds()
                        .into_iter()
                        .map(|kind| format!(r#""{}""#, kind.slug()))
                        .collect::<Vec<_>>()
                        .join(",");
                    format!(
                        r#"{{"slug":"{}","name":"{}","devices":[{}]}}"#,
                        p.slug(),
                        p.display_name(),
                        devices
                    )
                })
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
            let wiring = ctx.wiring_state.lock().unwrap().clone();
            let payload = dashboard_wiring_config(
                wiring.board,
                wiring.sensor_profile,
                &wiring.selected_devices,
                wiring.show_bus_labels,
            )
            .to_json();
            respond(
                &mut stream,
                "200 OK",
                "application/json; charset=utf-8",
                &payload,
            );
        }
        (_, "/api/wiring/svg") => {
            let wiring = ctx.wiring_state.lock().unwrap().clone();
            let cfg = dashboard_wiring_config(
                wiring.board,
                wiring.sensor_profile,
                &wiring.selected_devices,
                wiring.show_bus_labels,
            );
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

fn handle_sse_events(stream: &mut TcpStream, ctx: Arc<ServerContext>) {
    let header = "HTTP/1.1 200 OK\r\n\
        Content-Type: text/event-stream\r\n\
        Cache-Control: no-cache\r\n\
        Connection: keep-alive\r\n\
        Access-Control-Allow-Origin: *\r\n\
        \r\n";
    if stream.write_all(header.as_bytes()).is_err() {
        return;
    }

    let initial = ctx.latest_json.lock().unwrap().clone();
    let (tx, rx) = mpsc::sync_channel::<String>(32);
    ctx.sse_clients.lock().unwrap().push(tx);

    if stream
        .write_all(format!("data: {initial}\n\n").as_bytes())
        .is_err()
    {
        return;
    }

    for json in rx {
        if stream
            .write_all(format!("data: {json}\n\n").as_bytes())
            .is_err()
        {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Shutdown, TcpListener, TcpStream};
    use std::sync::Arc;
    use std::time::Duration;

    fn read_response(stream: &mut TcpStream) -> String {
        let mut buf = Vec::new();
        let mut chunk = [0u8; 1024];

        loop {
            match stream.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => buf.extend_from_slice(&chunk[..n]),
                Err(err)
                    if matches!(
                        err.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                    ) =>
                {
                    break
                }
                Err(err) => panic!("failed to read response: {err}"),
            }
        }

        String::from_utf8(buf).expect("response should be valid utf-8")
    }

    fn send_request(addr: std::net::SocketAddr, request: &str) -> String {
        let mut client = TcpStream::connect(addr).expect("client should connect");
        client
            .set_read_timeout(Some(Duration::from_millis(200)))
            .expect("client read timeout should be set");
        client
            .write_all(request.as_bytes())
            .expect("request should be written");
        read_response(&mut client)
    }

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

    #[test]
    fn parse_selected_devices_from_json_extracts_values() {
        assert_eq!(
            parse_selected_devices_from_json(r#"{"selected_devices":["bme280","servo","sgp30"]}"#),
            vec![
                "bme280".to_string(),
                "servo".to_string(),
                "sgp30".to_string()
            ]
        );
    }

    #[test]
    fn parse_json_string_field_handles_space_after_colon() {
        assert_eq!(
            parse_json_string_field(r#"{"sensor_profile": "climate"}"#, "sensor_profile"),
            Some("climate")
        );
    }

    #[test]
    fn sse_events_endpoint_streams_initial_state() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("listener should bind");
        let addr = listener
            .local_addr()
            .expect("listener should have local addr");
        let ctx = ServerContext::new(BoardProfile::OriginalEsp32);
        *ctx.latest_json.lock().unwrap() = r#"{"tick":1}"#.to_string();

        let (board_tx, board_rx) = mpsc::channel::<BoardProfile>();
        drop(board_rx);

        let ctx_for_thread = Arc::clone(&ctx);
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("test client should connect");
            handle_connection(stream, ctx_for_thread, board_tx);
        });

        let mut client = TcpStream::connect(addr).expect("client should connect");
        client
            .set_read_timeout(Some(Duration::from_millis(200)))
            .expect("client read timeout should be set");
        client
            .write_all(b"GET /api/events HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .expect("request should be written");

        let response = read_response(&mut client);
        assert!(response.contains("HTTP/1.1 200 OK\r\n"));
        assert!(response.contains("Content-Type: text/event-stream\r\n"));
        assert!(response.contains("data: {\"tick\":1}\n\n"));

        client
            .shutdown(Shutdown::Both)
            .expect("client should shut down cleanly");
        ctx.sse_clients.lock().unwrap().clear();

        server.join().expect("server thread should exit");
    }

    #[test]
    fn wiring_endpoint_updates_selected_devices() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("listener should bind");
        let addr = listener
            .local_addr()
            .expect("listener should have local addr");
        let ctx = ServerContext::new(BoardProfile::OriginalEsp32);

        let (board_tx, board_rx) = mpsc::channel::<BoardProfile>();
        drop(board_rx);

        let ctx_for_thread = Arc::clone(&ctx);
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("test client should connect");
            handle_connection(stream, ctx_for_thread, board_tx);
        });

        let body = r#"{"sensor_profile":"minimal","selected_devices":["bme280","servo"]}"#;
        let request = format!(
            "POST /api/wiring HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );

        let mut client = TcpStream::connect(addr).expect("client should connect");
        client
            .set_read_timeout(Some(Duration::from_millis(200)))
            .expect("client read timeout should be set");
        client
            .write_all(request.as_bytes())
            .expect("request should be written");

        let response = read_response(&mut client);
        assert!(response.contains("\"sensor_profile\":\"minimal\""));
        assert!(response.contains("\"selected_devices\":[\"bme280\",\"servo\"]"));
        assert!(response.contains("\"available_devices\":["));

        server.join().expect("server thread should exit");
    }

    #[test]
    fn wiring_endpoint_persists_bus_label_toggle_and_svg() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("listener should bind");
        let addr = listener
            .local_addr()
            .expect("listener should have local addr");
        let ctx = ServerContext::new(BoardProfile::OriginalEsp32);

        let (board_tx, board_rx) = mpsc::channel::<BoardProfile>();
        drop(board_rx);

        let ctx_for_thread = Arc::clone(&ctx);
        let server = thread::spawn(move || {
            for _ in 0..3 {
                let (stream, _) = listener.accept().expect("test client should connect");
                handle_connection(stream, Arc::clone(&ctx_for_thread), board_tx.clone());
            }
        });

        let body = r#"{"show_bus_labels":true}"#;
        let post_request = format!(
            "POST /api/wiring HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        let post_response = send_request(addr, &post_request);
        assert!(post_response.contains(r#""show_bus_labels":true"#));

        let wiring_response =
            send_request(addr, "GET /api/wiring HTTP/1.1\r\nHost: localhost\r\n\r\n");
        assert!(wiring_response.contains(r#""show_bus_labels":true"#));

        let svg_response = send_request(
            addr,
            "GET /api/wiring/svg HTTP/1.1\r\nHost: localhost\r\n\r\n",
        );
        assert!(svg_response.contains(r#"class="dev-pin""#));

        server.join().expect("server thread should exit");
    }

    #[test]
    fn device_simulation_rig_only_polls_selected_devices() {
        let mut rig = DeviceSimulationRig::new(BoardProfile::OriginalEsp32);
        let wiring_state = WiringState {
            board: BoardProfile::OriginalEsp32,
            sensor_profile: SensorProfile::Minimal,
            selected_devices: vec![DeviceKind::Bme280, DeviceKind::Lcd1602],
            show_bus_labels: false,
        };

        let state = rig.step(&wiring_state);

        assert_eq!(
            state.wiring.attached_devices,
            vec!["BME280 (0x77)".to_string(), "LCD1602 (0x27)".to_string()]
        );
        assert!(state.i2c.operation_count > 0);
        assert!(state
            .i2c
            .recent_operations
            .iter()
            .all(|line| line.contains("0x77") || line.contains("0x27")));
        assert!(state.climate.temperature_c.is_some());
        assert_eq!(state.climate.physical_lcd_frame[0].len(), 16);
        assert_eq!(state.distance.distance_mm, None);
        assert_eq!(state.imu.accel_mg, [0, 0, 0]);
        assert_eq!(state.light.lux_x100, 0);
        assert_eq!(state.camera.sequence, 0);
        assert_eq!(state.gas.co2_ppm, None);
        assert_eq!(state.rtc.datetime_str, "");
        assert_eq!(state.tof.distance_mm, None);
    }

    #[test]
    fn device_simulation_rig_keeps_bme280_data_when_lcd_is_disabled() {
        let mut rig = DeviceSimulationRig::new(BoardProfile::OriginalEsp32);
        let wiring_state = WiringState {
            board: BoardProfile::OriginalEsp32,
            sensor_profile: SensorProfile::Minimal,
            selected_devices: vec![DeviceKind::Bme280],
            show_bus_labels: false,
        };

        let state = rig.step(&wiring_state);

        assert_eq!(
            state.wiring.attached_devices,
            vec!["BME280 (0x77)".to_string()]
        );
        assert!(state.climate.temperature_c.is_some());
        assert!(state.climate.humidity_percent.is_some());
        assert!(state.climate.pressure_pa.is_some());
        assert_eq!(state.climate.app_frame, blank_lines());
        assert_eq!(state.climate.physical_lcd_frame, blank_lines());
    }

    #[test]
    fn device_simulation_rig_does_not_fabricate_climate_without_bme280() {
        let mut rig = DeviceSimulationRig::new(BoardProfile::OriginalEsp32);
        let wiring_state = WiringState {
            board: BoardProfile::OriginalEsp32,
            sensor_profile: SensorProfile::Minimal,
            selected_devices: vec![DeviceKind::Lcd1602],
            show_bus_labels: false,
        };

        let state = rig.step(&wiring_state);

        assert_eq!(
            state.wiring.attached_devices,
            vec!["LCD1602 (0x27)".to_string()]
        );
        assert_eq!(state.climate.temperature_c, None);
        assert_eq!(state.climate.humidity_percent, None);
        assert_eq!(state.climate.pressure_pa, None);
        assert_eq!(state.climate.physical_lcd_frame, blank_lines());
    }

    #[test]
    fn device_simulation_rig_reports_consistent_ds3231_address() {
        let mut rig = DeviceSimulationRig::new(BoardProfile::OriginalEsp32);
        let wiring_state = WiringState {
            board: BoardProfile::OriginalEsp32,
            sensor_profile: SensorProfile::ClimateStation,
            selected_devices: vec![DeviceKind::Ds3231],
            show_bus_labels: false,
        };

        let state = rig.step(&wiring_state);

        assert_eq!(
            state.wiring.attached_devices,
            vec!["DS3231 (0x68)".to_string()]
        );
        assert!(
            state
                .i2c
                .recent_operations
                .iter()
                .any(|line| line.contains("0x68")),
            "dashboard should render DS3231 traffic with the logical hardware address"
        );
        assert!(
            state
                .i2c
                .recent_operations
                .iter()
                .all(|line| !line.contains("0x69")),
            "dashboard should not expose the colliding hardware address"
        );
        assert!(!state.rtc.datetime_str.is_empty());
    }

    #[test]
    fn device_simulation_rig_resets_disabled_actuators() {
        let mut rig = DeviceSimulationRig::new(BoardProfile::OriginalEsp32);
        let active_wiring_state = WiringState {
            board: BoardProfile::OriginalEsp32,
            sensor_profile: SensorProfile::RobotBase,
            selected_devices: vec![
                DeviceKind::HcSr04,
                DeviceKind::Mpu6050,
                DeviceKind::Servo,
                DeviceKind::L298n,
            ],
            show_bus_labels: false,
        };

        let active_state = rig.step(&active_wiring_state);
        assert_ne!(active_state.servo.angle_degrees, 0);
        assert_eq!(active_state.motor_driver.left.direction, "forward");
        assert_eq!(active_state.motor_driver.left.duty_percent, 42);
        assert_eq!(active_state.motor_driver.right.direction, "forward");
        assert_eq!(active_state.motor_driver.right.duty_percent, 42);

        let disabled_wiring_state = WiringState {
            board: BoardProfile::OriginalEsp32,
            sensor_profile: SensorProfile::ClimateStation,
            selected_devices: vec![DeviceKind::Ds3231],
            show_bus_labels: false,
        };

        let disabled_state = rig.step(&disabled_wiring_state);
        assert_eq!(disabled_state.servo.angle_degrees, 0);
        assert_eq!(disabled_state.motor_driver.left.direction, "coast");
        assert_eq!(disabled_state.motor_driver.left.duty_percent, 0);
        assert_eq!(disabled_state.motor_driver.right.direction, "coast");
        assert_eq!(disabled_state.motor_driver.right.duty_percent, 0);
    }

    #[test]
    fn wiring_profiles_endpoint_lists_all_profiles() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("listener should bind");
        let addr = listener
            .local_addr()
            .expect("listener should have local addr");
        let ctx = ServerContext::new(BoardProfile::OriginalEsp32);

        let (board_tx, board_rx) = mpsc::channel::<BoardProfile>();
        drop(board_rx);

        let ctx_for_thread = Arc::clone(&ctx);
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("test client should connect");
            handle_connection(stream, ctx_for_thread, board_tx);
        });

        let response = send_request(
            addr,
            "GET /api/wiring/profiles HTTP/1.1\r\nHost: localhost\r\n\r\n",
        );
        assert!(response.contains("\"profiles\":["));
        assert!(response.contains("\"slug\":\"full\""));
        assert!(response.contains("\"slug\":\"climate\""));
        assert!(response.contains("\"slug\":\"robot\""));
        assert!(response.contains("\"slug\":\"minimal\""));
        assert!(response.contains(r#""devices":["bme280","lcd1602"]"#));

        server.join().expect("server thread should exit");
    }

    #[test]
    fn wiring_state_and_svg_reflect_explicit_selection_over_profile() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("listener should bind");
        let addr = listener
            .local_addr()
            .expect("listener should have local addr");
        let ctx = ServerContext::new(BoardProfile::OriginalEsp32);

        let (board_tx, board_rx) = mpsc::channel::<BoardProfile>();
        drop(board_rx);

        let ctx_for_thread = Arc::clone(&ctx);
        let server = thread::spawn(move || {
            for _ in 0..3 {
                let (stream, _) = listener.accept().expect("test client should connect");
                handle_connection(stream, Arc::clone(&ctx_for_thread), board_tx.clone());
            }
        });

        let body = r#"{"sensor_profile":"robot","selected_devices":["bme280","servo"]}"#;
        let post_request = format!(
            "POST /api/wiring HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        let post_response = send_request(addr, &post_request);
        assert!(post_response.contains("\"sensor_profile\":\"robot\""));
        assert!(post_response.contains("\"selected_devices\":[\"bme280\",\"servo\"]"));
        assert!(post_response.contains("\"devices\":["));
        assert!(post_response.contains("\"kind\":\"bme280\""));
        assert!(post_response.contains("\"kind\":\"servo\""));

        let wiring_response =
            send_request(addr, "GET /api/wiring HTTP/1.1\r\nHost: localhost\r\n\r\n");
        assert!(wiring_response.contains("\"selected_devices\":[\"bme280\",\"servo\"]"));
        assert!(wiring_response.contains("\"devices\":["));
        assert!(wiring_response.contains("\"kind\":\"bme280\""));
        assert!(wiring_response.contains("\"kind\":\"servo\""));

        let svg_response = send_request(
            addr,
            "GET /api/wiring/svg HTTP/1.1\r\nHost: localhost\r\n\r\n",
        );
        assert!(svg_response.contains("BME280"));
        assert!(svg_response.contains("Servo"));
        assert!(!svg_response.contains("MPU6050"));

        server.join().expect("server thread should exit");
    }
}
