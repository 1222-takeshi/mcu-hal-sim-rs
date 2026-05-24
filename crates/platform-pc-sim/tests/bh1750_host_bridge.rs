use hal_api::light::LightSensor;
use platform_esp32::bh1750::{Bh1750Sensor, BH1750_ADDRESS_LOW};
use platform_pc_sim::bh1750_mock::MockBh1750Device;
use platform_pc_sim::virtual_i2c::VirtualI2cBus;

#[test]
fn esp32_bh1750_driver_reads_lux_from_host_mock() {
    let bus = VirtualI2cBus::new();
    // 10000 lux×100 → 100.00 lx
    let device = MockBh1750Device::fixed(10_000);
    bus.attach_device(BH1750_ADDRESS_LOW, device);

    let mut sensor =
        Bh1750Sensor::new(bus.clone(), BH1750_ADDRESS_LOW).expect("BH1750 power-on failed");

    let reading = sensor.read_lux().expect("read_lux failed");
    assert!(reading.lux_x100 > 0, "expected non-zero lux reading");
    assert!(
        bus.attached_addresses().contains(&BH1750_ADDRESS_LOW),
        "device address not attached"
    );
}

#[test]
fn esp32_bh1750_driver_propagates_bus_error() {
    let bus = VirtualI2cBus::new();
    // No device attached at this address → reads return I2C error.
    let sensor = Bh1750Sensor::new(bus.clone(), BH1750_ADDRESS_LOW);
    assert!(
        sensor.is_err(),
        "expected power-on to fail when no device is attached"
    );
}
