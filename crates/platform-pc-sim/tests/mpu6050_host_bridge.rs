use hal_api::imu::ImuSensor;
use platform_esp32::mpu6050::{Mpu6050Sensor, MPU6050_ADDRESS_PRIMARY};
use platform_pc_sim::mpu6050_mock::{demo_raw_frames, MockMpu6050Device};
use platform_pc_sim::virtual_i2c::VirtualI2cBus;

#[test]
fn esp32_mpu6050_driver_runs_against_host_side_mock_device() {
    let bus = VirtualI2cBus::new();
    let device = MockMpu6050Device::new();
    let frames = demo_raw_frames();
    device.set_raw_frame(frames[1]);
    bus.attach_device(MPU6050_ADDRESS_PRIMARY, device.clone());

    let mut sensor = Mpu6050Sensor::new(bus.clone());
    let reading = sensor.read_imu().unwrap();

    assert_eq!(reading.accel_mg, [915, -156, 1015]);
    assert_eq!(reading.gyro_mdps, [0, 2000, -1000]);
    assert_eq!(reading.temperature_centi_celsius, Some(3673));
    assert_eq!(device.control_registers().power_management_1, 0x00);
    assert_eq!(device.control_registers().config, 0x03);
    assert_eq!(device.control_registers().gyro_config, 0x00);
    assert_eq!(device.control_registers().accel_config, 0x00);
    assert!(bus.attached_addresses().contains(&MPU6050_ADDRESS_PRIMARY));
}
