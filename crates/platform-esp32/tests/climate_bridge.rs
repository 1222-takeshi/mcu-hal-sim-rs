use core::cell::RefCell;
use std::rc::Rc;
use std::vec::Vec;

use core_app::climate_display::{ClimateDisplayApp, ClimateDisplayConfig};
use embedded_hal::delay::DelayNs;
use hal_api::error::I2cError;
use hal_api::i2c::I2cBus;
use platform_esp32::bme280::Bme280Sensor;
use platform_esp32::lcd1602::{Lcd1602Display, LCD1602_ADDRESS_PRIMARY};
use platform_esp32::shared_i2c::SharedI2cBus;

const REG_CHIP_ID: u8 = 0xD0;
const REG_CALIB_1_START: u8 = 0x88;
const REG_CALIB_2_START: u8 = 0xE1;
const REG_STATUS: u8 = 0xF3;
const REG_PRESS_MSB: u8 = 0xF7;

type BusLog = Rc<RefCell<Vec<(u8, Vec<u8>)>>>;

#[derive(Clone, Default)]
struct MultiplexedI2c {
    writes: BusLog,
    responses: BusLog,
}

impl MultiplexedI2c {
    fn with_bme280_defaults() -> Self {
        let bus = Self::default();
        bus.set_response(REG_CHIP_ID, &[0x60]);
        bus.set_response(REG_STATUS, &[0x00]);
        bus.set_response(
            REG_CALIB_1_START,
            &[
                0x70, 0x6B, 0x43, 0x67, 0x18, 0xFC, 0x7D, 0x8E, 0x43, 0xD6, 0xD0, 0x0B, 0x27, 0x0B,
                0x8C, 0x00, 0xF9, 0xFF, 0x8C, 0x3C, 0xF8, 0xC6, 0x70, 0x17, 0x00, 0x4B,
            ],
        );
        bus.set_response(
            REG_CALIB_2_START,
            &[0x6A, 0x01, 0x00, 0x14, 0x25, 0x03, 0x1E],
        );
        bus.set_response(
            REG_PRESS_MSB,
            &[0x65, 0x5A, 0xC0, 0x7E, 0xED, 0x00, 0x89, 0x98],
        );
        bus
    }

    fn set_response(&self, register: u8, bytes: &[u8]) {
        let mut responses = self.responses.borrow_mut();
        responses.retain(|(candidate, _)| *candidate != register);
        responses.push((register, bytes.to_vec()));
    }
}

impl I2cBus for MultiplexedI2c {
    type Error = I2cError;

    fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        self.writes.borrow_mut().push((addr, bytes.to_vec()));
        Ok(())
    }

    fn read(&mut self, _addr: u8, _buffer: &mut [u8]) -> Result<(), Self::Error> {
        Err(I2cError::BusError)
    }

    fn write_read(&mut self, addr: u8, bytes: &[u8], buffer: &mut [u8]) -> Result<(), Self::Error> {
        self.writes.borrow_mut().push((addr, bytes.to_vec()));
        let register = *bytes.first().ok_or(I2cError::BusError)?;
        let response = self
            .responses
            .borrow()
            .iter()
            .find(|(candidate, _)| *candidate == register)
            .map(|(_, data)| data.clone())
            .ok_or(I2cError::InvalidAddress)?;
        if response.len() != buffer.len() {
            return Err(I2cError::BusError);
        }
        buffer.copy_from_slice(&response);
        Ok(())
    }
}

#[derive(Default)]
struct NoopDelay;

impl DelayNs for NoopDelay {
    fn delay_ns(&mut self, _ns: u32) {}
    fn delay_us(&mut self, _us: u32) {}
    fn delay_ms(&mut self, _ms: u32) {}
}

#[test]
fn climate_display_app_uses_shared_i2c_for_sensor_and_display() {
    let inner = RefCell::new(MultiplexedI2c::with_bme280_defaults());
    let writes = inner.borrow().writes.clone();

    let sensor = Bme280Sensor::new(SharedI2cBus::new(&inner));
    let display = Lcd1602Display::new(SharedI2cBus::new(&inner), NoopDelay);
    let mut app = ClimateDisplayApp::new_with_config(
        sensor,
        display,
        ClimateDisplayConfig {
            refresh_period_ticks: 1,
        },
    );

    app.tick().unwrap();

    assert!(writes
        .borrow()
        .iter()
        .any(|(addr, bytes)| *addr == 0x77 && bytes.as_slice() == [0xF2, 0x01]));
    assert!(writes
        .borrow()
        .iter()
        .any(|(addr, _)| *addr == LCD1602_ADDRESS_PRIMARY));
}
