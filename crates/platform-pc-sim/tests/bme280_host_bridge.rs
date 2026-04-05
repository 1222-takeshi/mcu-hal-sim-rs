use core_app::climate_display::{ClimateDisplayApp, ClimateDisplayConfig};
use platform_esp32::bme280::{Bme280Config, Bme280Sensor, BME280_ADDRESS_PRIMARY};
use platform_pc_sim::bme280_mock::MockBme280Device;
use platform_pc_sim::climate_sim::TerminalDisplay16x2;
use platform_pc_sim::virtual_i2c::{VirtualI2cBus, VirtualI2cOperation};

#[test]
fn esp32_bme280_driver_runs_against_host_side_mock_device() {
    let bus = VirtualI2cBus::new();
    let device = MockBme280Device::new();
    bus.attach_device(BME280_ADDRESS_PRIMARY, device.clone());

    let sensor = Bme280Sensor::new_with_config(
        bus.clone(),
        Bme280Config {
            address: BME280_ADDRESS_PRIMARY,
            ctrl_hum: 0x01,
            ctrl_meas: 0x27,
            config: 0x10,
        },
    );
    let display = TerminalDisplay16x2::new();
    let display_observer = display.clone();
    let mut app = ClimateDisplayApp::new_with_config(
        sensor,
        display,
        ClimateDisplayConfig {
            refresh_period_ticks: 1,
            refresh_on_first_tick: true,
        },
    );

    app.tick().unwrap();

    let frame = display_observer.last_ascii().unwrap();
    assert!(frame.contains("Temp"));
    assert!(frame.contains("Hum"));
    assert_eq!(
        device.control_registers(),
        platform_pc_sim::bme280_mock::Bme280ControlRegisters {
            ctrl_hum: 0x01,
            ctrl_meas: 0x27,
            config: 0x10,
        }
    );
    assert!(bus
        .operations()
        .iter()
        .any(|operation| matches!(operation, VirtualI2cOperation::WriteRead { addr, .. } if *addr == BME280_ADDRESS_PRIMARY)));
}
