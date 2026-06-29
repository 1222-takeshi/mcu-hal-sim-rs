use core::str;
use std::collections::VecDeque;
use std::fmt::Write as FmtWrite;

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
use platform_pc_sim::ssd1306_mock::MockSsd1306TextDisplay;
use platform_pc_sim::virtual_i2c::{VirtualI2cBus, VirtualI2cOperation};
use platform_pc_sim::vl53l0x_mock::MockVl53l0xDevice;
use platform_pc_sim::web_dashboard::{
    CameraPanelState, ClimatePanelState, DeviceDashboardState, DiagEvent, DiagnosticsPanelState,
    DistancePanelState, GasPanelState, I2cPanelState, ImuPanelState, LightPanelState,
    MotorChannelState, MotorDriverPanelState, OledPanelState, RtcPanelState, ServoPanelState,
    TofPanelState, WiringPanelState,
};
use platform_pc_sim::wiring_config::{
    normalize_supported_device_selection, ConnectionType, DeviceKind, WiringConfig,
};
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

use super::{dashboard_wiring_config, WiringState};

/// DS3231 and MPU6050 share hardware address 0x68. The simulator attaches DS3231
/// at this offset address to avoid collision; display helpers translate it back.
pub(super) const DS3231_SIM_ADDRESS: u8 = DS3231_ADDRESS + 1;

// ── NoopDelay ──────────────────────────────────────────────────────────────

#[derive(Default)]
pub(super) struct NoopDelay;

impl DelayNs for NoopDelay {
    fn delay_ns(&mut self, _ns: u32) {}
    fn delay_us(&mut self, _us: u32) {}
    fn delay_ms(&mut self, _ms: u32) {}
}

// ── Type aliases ───────────────────────────────────────────────────────────

pub(super) type ServoRig = ServoDriver<MockPwmOutput>;
pub(super) type MotorChannelRig = L298nChannel<MockPin, MockPin, MockPwmOutput>;
pub(super) type MotorDriverRig = L298nDualDriver<MotorChannelRig, MotorChannelRig>;

// ── DeviceSimulationRig ────────────────────────────────────────────────────

pub(super) struct DeviceSimulationRig {
    pub board: BoardProfile,
    pub bus: VirtualI2cBus,
    pub bme280: MockBme280Device,
    pub lcd: MockLcd1602Device,
    pub mpu6050: MockMpu6050Device,
    pub climate_sensor: Bme280Sensor<VirtualI2cBus>,
    pub app:
        ClimateDisplayApp<Bme280Sensor<VirtualI2cBus>, Lcd1602Display<VirtualI2cBus, NoopDelay>>,
    pub bme280_samples: Vec<[u8; 8]>,
    pub bme280_sample_index: usize,
    pub distance_sensor: HcSr04Sensor<MockHcSr04Device>,
    pub imu_sensor: Mpu6050Sensor<VirtualI2cBus>,
    pub imu_frames: Vec<[u8; 14]>,
    pub imu_frame_index: usize,
    pub servo: ServoRig,
    pub motor_driver: MotorDriverRig,
    pub tick: u32,
    pub last_distance_mm: Option<u32>,
    pub last_imu: Option<hal_api::imu::ImuReading>,
    pub light_sensor: Bh1750Sensor<VirtualI2cBus>,
    pub bh1750_mock: platform_pc_sim::bh1750_mock::MockBh1750Device,
    pub camera: MockCamera,
    pub last_lux_x100: u32,
    pub last_camera_sequence: u32,
    pub ds3231_mock: MockDs3231Device,
    pub ds3231_timestamps: Vec<platform_pc_sim::ds3231_mock::MockRtcTimestamp>,
    pub ds3231_ts_index: usize,
    pub rtc_sensor: Ds3231Sensor<VirtualI2cBus>,
    pub sgp30_mock: MockSgp30Device,
    pub sgp30_sensor: Sgp30Sensor<VirtualI2cBus>,
    pub vl53l0x_mock: MockVl53l0xDevice,
    pub tof_sensor: Vl53l0xSensor<VirtualI2cBus>,
    pub ssd1306_display: MockSsd1306TextDisplay,
    pub last_gas: Option<hal_api::gas::GasReading>,
    pub last_rtc_str: String,
    pub last_tof_mm: Option<u32>,
    pub last_oled_frame: Option<[String; 2]>,
    /// Ring buffer of diagnostic events (capacity 20, most-recent-last).
    pub diag_ring: VecDeque<DiagEvent>,
    /// Cumulative diagnostic event counter.
    pub diag_event_count: u32,
    /// Monotonic start time for elapsed-ms timestamps in diag events.
    pub start_instant: std::time::Instant,
    /// Device selection from the previous tick — used to detect toggle events.
    pub last_selected_devices: Vec<DeviceKind>,
}

impl DeviceSimulationRig {
    pub fn new(board: BoardProfile) -> Self {
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
        let ssd1306_display = MockSsd1306TextDisplay::new();

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
            ssd1306_display,
            last_gas: None,
            last_rtc_str: String::new(),
            last_tof_mm: None,
            last_oled_frame: None,
            diag_ring: VecDeque::new(),
            diag_event_count: 0,
            start_instant: std::time::Instant::now(),
            last_selected_devices: vec![],
        }
    }

    pub fn sync_selected_devices(&mut self, selected_devices: &[DeviceKind]) {
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

    pub fn push_diag(&mut self, severity: &str, msg: String) {
        const MAX_RING: usize = 20;
        self.diag_event_count = self.diag_event_count.saturating_add(1);
        if self.diag_ring.len() >= MAX_RING {
            self.diag_ring.pop_front();
        }
        self.diag_ring.push_back(DiagEvent {
            elapsed_ms: self.start_instant.elapsed().as_millis() as u64,
            severity: severity.to_string(),
            message: msg,
        });
    }

    pub fn step(&mut self, wiring_state: &WiringState) -> DeviceDashboardState {
        self.tick = self.tick.wrapping_add(1);
        let tick = self.tick;
        let selected_devices = normalize_supported_device_selection(
            wiring_state.board,
            &wiring_state.selected_devices,
        );
        self.sync_selected_devices(&selected_devices);
        self.bus.clear_operations();

        // Detect device toggle events compared to the previous tick.
        if tick > 1 {
            let enabled_events: Vec<String> = selected_devices
                .iter()
                .filter(|k| !self.last_selected_devices.contains(k))
                .map(|k| format!("{} enabled", k.slug()))
                .collect();
            let disabled_events: Vec<String> = self
                .last_selected_devices
                .iter()
                .filter(|k| !selected_devices.contains(k))
                .map(|k| format!("{} disabled", k.slug()))
                .collect();
            for msg in enabled_events.into_iter().chain(disabled_events) {
                self.push_diag("info", msg);
            }
        }
        self.last_selected_devices = selected_devices.clone();

        let is_enabled = |kind: DeviceKind| selected_devices.contains(&kind);
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
        if !is_enabled(DeviceKind::Ssd1306) {
            self.last_oled_frame = None;
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
            match self.light_sensor.read_lux() {
                Ok(reading) => self.last_lux_x100 = reading.lux_x100,
                Err(_) => self.push_diag("error", "[bh1750] read_lux error".into()),
            }
        }

        if is_enabled(DeviceKind::Esp32Cam) && (tick == 1 || tick % 7 == 0) {
            match self.camera.capture_frame() {
                Ok(frame) => self.last_camera_sequence = frame.sequence,
                Err(_) => self.push_diag("error", "[esp32cam] capture_frame error".into()),
            }
        }

        // Poll SGP30 gas sensor every 11 ticks
        if is_enabled(DeviceKind::Sgp30) && (tick == 1 || tick % 11 == 0) {
            match self.sgp30_sensor.read_gas() {
                Ok(reading) => self.last_gas = Some(reading),
                Err(_) => self.push_diag("error", "[sgp30] read_gas error".into()),
            }
        }

        // Poll DS3231 RTC every tick so /api/state recent_operations always reflects the
        // currently selected simulator address.
        if is_enabled(DeviceKind::Ds3231) {
            let ts = self.ds3231_timestamps[self.ds3231_ts_index];
            self.ds3231_ts_index = (self.ds3231_ts_index + 1) % self.ds3231_timestamps.len();
            self.ds3231_mock.set_timestamp(ts);
            match self.rtc_sensor.read_datetime() {
                Ok(dt) => {
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
                Err(_) => self.push_diag("error", "[ds3231] read_datetime error".into()),
            }
        }

        // Poll VL53L0X ToF sensor every 4 ticks
        if is_enabled(DeviceKind::Vl53l0x) && (tick == 1 || tick % 4 == 0) {
            match self.tof_sensor.read_distance() {
                Ok(reading) => self.last_tof_mm = Some(reading.distance_mm),
                Err(_) => self.push_diag("error", "[vl53l0x] read_distance error".into()),
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
            &selected_devices,
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
                .unwrap_or_else(|| blank_lines().map(|s| s.to_owned()))
        } else {
            climate
                .and_then(|reading| frame_from_reading(reading).ok())
                .map(frame_to_lines)
                .unwrap_or_else(|| blank_lines().map(|s| s.to_owned()))
        };
        let imu = self
            .last_imu
            .unwrap_or_else(|| hal_api::imu::ImuReading::new([0, 0, 0], [0, 0, 0], None));
        let lcd_frame = if bme280_enabled && lcd_enabled {
            physical_lcd_frame
        } else {
            blank_lines().map(|s| s.to_owned())
        };

        // Render climate frame to SSD1306 display every 5 ticks when both are enabled
        if is_enabled(DeviceKind::Ssd1306) && bme280_enabled && (tick == 1 || tick % 5 == 0) {
            if let Some(reading) = climate {
                if let Ok(frame) = frame_from_reading(reading) {
                    let _ = hal_api::display::TextDisplay16x2::render(
                        &mut self.ssd1306_display,
                        &frame,
                    );
                    self.last_oled_frame = self.ssd1306_display.last_frame();
                }
            }
        }

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
                    blank_lines().map(|s| s.to_owned())
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
                selected_devices: selected_devices
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
            oled: OledPanelState {
                frame: self
                    .last_oled_frame
                    .clone()
                    .unwrap_or_else(|| ["".to_string(), "".to_string()]),
                sensor_name: "SSD1306".to_string(),
            },
            diagnostics: DiagnosticsPanelState {
                recent_events: self.diag_ring.iter().rev().cloned().collect(),
                event_count: self.diag_event_count,
            },
        }
    }
}

// ── Display helpers ─────────────────────────────────────────────────────────

pub(super) fn blank_lines() -> [&'static str; 2] {
    ["                ", "                "]
}

fn frame_to_lines(frame: hal_api::display::TextFrame16x2) -> [String; 2] {
    [0, 1].map(|row| {
        str::from_utf8(frame.line(row))
            .unwrap_or("????????????????")
            .to_owned()
    })
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

// ── Motor / servo helpers ──────────────────────────────────────────────────

pub(super) fn distance_to_servo_angle(distance_mm: u32) -> u16 {
    let clamped = distance_mm.clamp(80, 360) - 80;
    ((clamped * 180) / 280) as u16
}

pub(super) fn motor_commands_from_state(
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

// ── I2C address display helpers ────────────────────────────────────────────

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
