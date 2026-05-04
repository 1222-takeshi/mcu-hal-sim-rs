//! ESP32 Bridge tests: BH1750 light sensor と SSD1306 OLED display の
//! platform-esp32 経由での動作を検証します。

use core::cell::RefCell;
use std::rc::Rc;
use std::vec::Vec;

use hal_api::error::I2cError;
use hal_api::i2c::I2cBus;
use hal_api::light::LightSensor;
use platform_esp32::bh1750::{Bh1750Sensor, BH1750_ADDRESS_LOW};
use platform_esp32::ssd1306::{Ssd1306Display, SSD1306_ADDRESS_DEFAULT};

// ---- Stub I2C ----

type WriteLog = Rc<RefCell<Vec<(u8, Vec<u8>)>>>;

#[derive(Clone, Default)]
struct StubI2c {
    writes: WriteLog,
    next_read: Rc<RefCell<Vec<u8>>>,
}

impl StubI2c {
    fn set_next_read(&self, bytes: &[u8]) {
        *self.next_read.borrow_mut() = bytes.to_vec();
    }
}

impl I2cBus for StubI2c {
    type Error = I2cError;

    fn write(&mut self, addr: u8, data: &[u8]) -> Result<(), I2cError> {
        self.writes.borrow_mut().push((addr, data.to_vec()));
        Ok(())
    }

    fn read(&mut self, _addr: u8, buf: &mut [u8]) -> Result<(), I2cError> {
        let src = self.next_read.borrow();
        for (dst, src) in buf.iter_mut().zip(src.iter()) {
            *dst = *src;
        }
        Ok(())
    }

    fn write_read(&mut self, addr: u8, write: &[u8], buf: &mut [u8]) -> Result<(), I2cError> {
        self.writes.borrow_mut().push((addr, write.to_vec()));
        let src = self.next_read.borrow();
        for (dst, src) in buf.iter_mut().zip(src.iter()) {
            *dst = *src;
        }
        Ok(())
    }
}

// ---- BH1750 bridge test ----

#[test]
fn bh1750_bridge_reads_lux_via_esp32_module() {
    let bus = StubI2c::default();

    // raw = 720 → lux×100 = 720 * 500 / 6 = 60000 (600 lx)
    bus.set_next_read(&[0x02, 0xD0]); // 720 in big-endian

    let mut sensor = Bh1750Sensor::new(bus, BH1750_ADDRESS_LOW).unwrap();
    let reading = sensor.read_lux().unwrap();
    assert_eq!(reading.lux_x100, 60000);
}

#[test]
fn bh1750_bridge_uses_correct_i2c_address() {
    assert_eq!(BH1750_ADDRESS_LOW, 0x23);
}

// ---- SSD1306 bridge test ----

#[test]
fn ssd1306_bridge_initializes_via_esp32_module() {
    let bus = StubI2c::default();
    let result = Ssd1306Display::new(bus, SSD1306_ADDRESS_DEFAULT);
    assert!(result.is_ok(), "SSD1306 should initialize without error");
}

#[test]
fn ssd1306_bridge_uses_correct_i2c_address() {
    assert_eq!(SSD1306_ADDRESS_DEFAULT, 0x3C);
}

#[test]
fn ssd1306_bridge_renders_frame() {
    use hal_api::display::{TextDisplay16x2, TextFrame16x2};
    let bus = StubI2c::default();
    let mut display = Ssd1306Display::new(bus, SSD1306_ADDRESS_DEFAULT).unwrap();
    let frame = TextFrame16x2::from_lines("BH1750  600 lx  ", "SSD1306 ready   ");
    assert!(display.render(&frame).is_ok());
}

// ---- DHT22 bridge test ----

#[test]
fn dht22_bridge_stub_returns_not_initialized() {
    use embedded_hal::digital::ErrorType;
    use platform_esp32::dht22::Esp32Dht22RawDevice;

    struct StubPin;

    impl ErrorType for StubPin {
        type Error = core::convert::Infallible;
    }

    impl embedded_hal::digital::InputPin for StubPin {
        fn is_high(&mut self) -> Result<bool, Self::Error> {
            Ok(true)
        }
        fn is_low(&mut self) -> Result<bool, Self::Error> {
            Ok(false)
        }
    }

    impl embedded_hal::digital::OutputPin for StubPin {
        fn set_high(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
        fn set_low(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    struct NoopDelay;
    impl embedded_hal::delay::DelayNs for NoopDelay {
        fn delay_ns(&mut self, _ns: u32) {}
    }

    let mut dev = Esp32Dht22RawDevice::new(StubPin, NoopDelay);
    let result = platform_esp32::dht22::Dht22RawDevice::read_raw_bytes(&mut dev);
    assert!(matches!(
        result,
        Err(hal_api::error::SensorError::NotInitialized)
    ));
}
