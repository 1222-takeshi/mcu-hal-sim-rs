use hal_api::rtc::RtcSensor;
use platform_esp32::ds3231::{Ds3231Sensor, DS3231_ADDRESS};
use platform_pc_sim::ds3231_mock::{demo_timestamps, MockDs3231Device, MockRtcTimestamp};
use platform_pc_sim::virtual_i2c::VirtualI2cBus;

#[test]
fn esp32_ds3231_driver_reads_datetime_from_host_mock() {
    let bus = VirtualI2cBus::new();
    let device = MockDs3231Device::new();
    // Set a known timestamp: 2025-05-04 09:00:00
    device.set_timestamp(MockRtcTimestamp::from_decimal(0, 0, 9, 1, 4, 5, 25));
    bus.attach_device(DS3231_ADDRESS, device.clone());

    let mut sensor = Ds3231Sensor::new(bus.clone(), DS3231_ADDRESS);
    let dt = sensor.read_datetime().expect("read_datetime failed");

    assert_eq!(dt.hour, 9);
    assert_eq!(dt.minute, 0);
    assert_eq!(dt.month, 5);
    assert_eq!(dt.day, 4);
    assert_eq!(dt.year_offset, 25);
    assert!(
        bus.attached_addresses().contains(&DS3231_ADDRESS),
        "device address not attached"
    );
}

#[test]
fn esp32_ds3231_driver_sequences_through_demo_timestamps() {
    let bus = VirtualI2cBus::new();
    let device = MockDs3231Device::new();
    let timestamps = demo_timestamps();
    device.set_timestamp(timestamps[1]);
    bus.attach_device(DS3231_ADDRESS, device);

    let mut sensor = Ds3231Sensor::new(bus.clone(), DS3231_ADDRESS);
    let dt = sensor.read_datetime().expect("read_datetime failed");

    // timestamps[1] = sec=15, min=0, hour=9 (from_decimal(15, 0, 9, ...))
    assert_eq!(dt.hour, 9);
    assert_eq!(dt.minute, 0);
    assert_eq!(dt.second, 15);
    assert_eq!(dt.month, 5);
}

#[test]
fn esp32_ds3231_driver_propagates_bus_error() {
    let bus = VirtualI2cBus::new();
    // No device attached → read_datetime returns an error.
    let mut sensor = Ds3231Sensor::new(bus, DS3231_ADDRESS);
    assert!(
        sensor.read_datetime().is_err(),
        "expected error when no device is attached"
    );
}
