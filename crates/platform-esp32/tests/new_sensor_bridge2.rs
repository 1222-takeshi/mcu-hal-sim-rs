//! ESP32 Bridge tests: DS3231 RTC, SGP30 gas sensor, VL53L0X ToF distance sensor の
//! platform-esp32 経由での動作を検証します。

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::vec::Vec;

use hal_api::distance::DistanceSensor;
use hal_api::error::I2cError;
use hal_api::gas::GasSensor;
use hal_api::i2c::I2cBus;
use hal_api::rtc::RtcSensor;
use platform_esp32::ds3231::{Ds3231Sensor, DS3231_ADDRESS};
use platform_esp32::sgp30::{Sgp30Sensor, SGP30_ADDRESS};
use platform_esp32::vl53l0x::{Vl53l0xSensor, VL53L0X_ADDRESS};

// ---- Sequential I2C stub ----
// Responses are returned in FIFO order for every read() or write_read() call.

type WriteLog = Rc<RefCell<Vec<(u8, Vec<u8>)>>>;

#[derive(Clone, Default)]
struct SequentialI2c {
    writes: WriteLog,
    read_responses: Rc<RefCell<VecDeque<Vec<u8>>>>,
}

impl SequentialI2c {
    fn push_response(&self, bytes: &[u8]) {
        self.read_responses.borrow_mut().push_back(bytes.to_vec());
    }

    fn pop_response(&self, buf: &mut [u8]) -> Result<(), I2cError> {
        let mut q = self.read_responses.borrow_mut();
        let response = q.pop_front().ok_or(I2cError::BusError)?;
        for (dst, src) in buf.iter_mut().zip(response.iter()) {
            *dst = *src;
        }
        Ok(())
    }
}

impl I2cBus for SequentialI2c {
    type Error = I2cError;

    fn write(&mut self, addr: u8, data: &[u8]) -> Result<(), I2cError> {
        self.writes.borrow_mut().push((addr, data.to_vec()));
        Ok(())
    }

    fn read(&mut self, _addr: u8, buf: &mut [u8]) -> Result<(), I2cError> {
        self.pop_response(buf)
    }

    fn write_read(&mut self, addr: u8, write: &[u8], buf: &mut [u8]) -> Result<(), I2cError> {
        self.writes.borrow_mut().push((addr, write.to_vec()));
        self.pop_response(buf)
    }
}

// ---- DS3231 bridge tests ----

#[test]
fn ds3231_bridge_reads_datetime_via_esp32_module() {
    let bus = SequentialI2c::default();
    // Encode 2025-05-04 09:30:00 in BCD (24h mode)
    // [sec, min, hour, dow, day, month, year_offset]
    bus.push_response(&[0x00, 0x30, 0x09, 0x01, 0x04, 0x05, 0x25]);

    let mut sensor = Ds3231Sensor::new(bus, DS3231_ADDRESS);
    let dt = sensor.read_datetime().unwrap();

    assert_eq!(dt.second, 0);
    assert_eq!(dt.minute, 30);
    assert_eq!(dt.hour, 9);
    assert_eq!(dt.day, 4);
    assert_eq!(dt.month, 5);
    assert_eq!(dt.year(), 2025);
}

#[test]
fn ds3231_bridge_uses_correct_i2c_address() {
    assert_eq!(DS3231_ADDRESS, 0x68);
}

#[test]
fn ds3231_bridge_sends_register_pointer_on_read() {
    let bus = SequentialI2c::default();
    bus.push_response(&[0x00, 0x00, 0x12, 0x01, 0x01, 0x01, 0x00]);

    let writes = bus.writes.clone();
    let mut sensor = Ds3231Sensor::new(bus, DS3231_ADDRESS);
    sensor.read_datetime().unwrap();

    // write_read should send [0x00] as the register pointer
    let log = writes.borrow();
    assert!(!log.is_empty());
    assert_eq!(log[0].1, vec![0x00]);
}

// ---- SGP30 bridge tests ----

#[test]
fn sgp30_bridge_init_and_read_via_esp32_module() {
    let bus = SequentialI2c::default();
    // Measurement response: CO₂=450ppm, VOC=30ppb (CRC bytes ignored)
    // buf = [co2_h, co2_l, crc, voc_h, voc_l, crc]
    bus.push_response(&[0x01, 0xC2, 0x00, 0x00, 0x1E, 0x00]);

    let mut sensor = Sgp30Sensor::new(bus, SGP30_ADDRESS).unwrap();
    let reading = sensor.read_gas().unwrap();

    assert_eq!(reading.co2_ppm, 450);
    assert_eq!(reading.voc_ppb, 30);
}

#[test]
fn sgp30_bridge_uses_correct_i2c_address() {
    assert_eq!(SGP30_ADDRESS, 0x58);
}

#[test]
fn sgp30_bridge_sends_init_command_on_new() {
    let bus = SequentialI2c::default();
    let writes = bus.writes.clone();
    // init only; no measurement needed
    let _sensor = Sgp30Sensor::new(bus, SGP30_ADDRESS).unwrap();

    // First write must be the init_air_quality command [0x20, 0x03]
    let log = writes.borrow();
    assert!(!log.is_empty());
    assert_eq!(log[0], (SGP30_ADDRESS, vec![0x20, 0x03]));
}

// ---- VL53L0X bridge tests ----

#[test]
fn vl53l0x_bridge_reads_distance_via_esp32_module() {
    let bus = SequentialI2c::default();
    // Sequence of write_read responses:
    // 1) verify_identity: model ID = 0xEE
    // 2) poll interrupt status: ready (bit0 set)
    // 3) read distance: 1000 mm (0x03, 0xE8)
    bus.push_response(&[0xEE]); // model ID check
    bus.push_response(&[0x01]); // interrupt status = ready
    bus.push_response(&[0x03, 0xE8]); // distance = 1000 mm

    let mut sensor = Vl53l0xSensor::new(bus, VL53L0X_ADDRESS).unwrap();
    let reading = sensor.read_distance().unwrap();

    assert_eq!(reading.distance_mm, 1000);
}

#[test]
fn vl53l0x_bridge_uses_correct_i2c_address() {
    assert_eq!(VL53L0X_ADDRESS, 0x29);
}

#[test]
fn vl53l0x_bridge_rejects_wrong_model_id() {
    let bus = SequentialI2c::default();
    bus.push_response(&[0xFF]); // wrong model ID

    let result = Vl53l0xSensor::new(bus, VL53L0X_ADDRESS);
    assert!(result.is_err(), "wrong model ID should fail initialization");
}
