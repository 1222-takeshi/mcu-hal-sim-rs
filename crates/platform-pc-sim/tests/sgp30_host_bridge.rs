use hal_api::gas::GasSensor;
use platform_esp32::sgp30::{Sgp30Sensor, SGP30_ADDRESS};
use platform_pc_sim::sgp30_mock::{demo_gas_readings, MockSgp30Device};
use platform_pc_sim::virtual_i2c::VirtualI2cBus;

#[test]
fn esp32_sgp30_driver_reads_gas_from_host_mock() {
    let bus = VirtualI2cBus::new();
    let device = MockSgp30Device::new();
    bus.attach_device(SGP30_ADDRESS, device.clone());

    let mut sensor = Sgp30Sensor::new(bus.clone(), SGP30_ADDRESS).expect("SGP30 init failed");
    let reading = sensor.read_gas().expect("read_gas failed");

    // First demo reading: co2=400 ppm, voc=0 ppb
    let expected = demo_gas_readings()[0];
    assert_eq!(reading.co2_ppm, expected.co2_ppm);
    assert_eq!(reading.voc_ppb, expected.voc_ppb);
    assert!(
        bus.attached_addresses().contains(&SGP30_ADDRESS),
        "device address not attached"
    );
}

#[test]
fn esp32_sgp30_driver_init_sends_init_command() {
    let bus = VirtualI2cBus::new();
    let device = MockSgp30Device::new();
    bus.attach_device(SGP30_ADDRESS, device.clone());

    let _sensor = Sgp30Sensor::new(bus, SGP30_ADDRESS).expect("SGP30 init failed");

    assert!(
        device.is_initialized(),
        "expected init_air_quality command to be sent on construction"
    );
}

#[test]
fn esp32_sgp30_driver_propagates_bus_error() {
    let bus = VirtualI2cBus::new();
    // No device attached → init write fails.
    let result = Sgp30Sensor::new(bus, SGP30_ADDRESS);
    assert!(result.is_err(), "expected error when no device is attached");
}
