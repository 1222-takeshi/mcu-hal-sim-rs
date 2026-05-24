use hal_api::distance::DistanceSensor;
use platform_esp32::vl53l0x::{Vl53l0xSensor, VL53L0X_ADDRESS};
use platform_pc_sim::virtual_i2c::VirtualI2cBus;
use platform_pc_sim::vl53l0x_mock::{demo_distances_mm, MockVl53l0xDevice};

#[test]
fn esp32_vl53l0x_driver_reads_distance_from_host_mock() {
    let bus = VirtualI2cBus::new();
    let device = MockVl53l0xDevice::new();
    bus.attach_device(VL53L0X_ADDRESS, device.clone());

    let mut sensor = Vl53l0xSensor::new(bus.clone(), VL53L0X_ADDRESS).expect("VL53L0X init failed");
    let reading = sensor.read_distance().expect("read_distance failed");

    // First demo distance: 1500 mm
    assert_eq!(reading.distance_mm, demo_distances_mm()[0]);
    assert!(
        bus.attached_addresses().contains(&VL53L0X_ADDRESS),
        "device address not attached"
    );
}

#[test]
fn esp32_vl53l0x_driver_custom_distance() {
    let bus = VirtualI2cBus::new();
    // Use a single-element looping mock so advance() always returns 250.
    let device = MockVl53l0xDevice::looping(vec![250]);
    bus.attach_device(VL53L0X_ADDRESS, device);

    let mut sensor = Vl53l0xSensor::new(bus, VL53L0X_ADDRESS).expect("VL53L0X init failed");
    let reading = sensor.read_distance().expect("read_distance failed");
    assert_eq!(reading.distance_mm, 250);
}

#[test]
fn esp32_vl53l0x_driver_identity_check_fails_without_device() {
    let bus = VirtualI2cBus::new();
    // No device → identity check fails.
    let result = Vl53l0xSensor::new(bus, VL53L0X_ADDRESS);
    assert!(
        result.is_err(),
        "expected error when no device is attached (identity check should fail)"
    );
}
