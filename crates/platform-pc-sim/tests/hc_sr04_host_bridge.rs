use hal_api::distance::DistanceSensor;
use platform_esp32::hc_sr04::HcSr04Sensor;
use platform_pc_sim::hc_sr04_mock::{demo_echo_pulses_us, MockHcSr04Device};

#[test]
fn esp32_hc_sr04_driver_runs_against_host_side_mock_device() {
    let device = MockHcSr04Device::looping(demo_echo_pulses_us());
    let device_handle = device.clone();
    let mut sensor = HcSr04Sensor::new(device);

    let reading = sensor.read_distance().unwrap();

    assert_eq!(reading.distance_mm, 180);
    assert_eq!(device_handle.trigger_count(), 1);
}
