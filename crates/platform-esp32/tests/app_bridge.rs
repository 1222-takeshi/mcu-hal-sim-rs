use core::convert::Infallible;
use std::cell::RefCell;
use std::rc::Rc;

use core_app::App;
use embedded_hal::digital::{ErrorType as DigitalErrorType, OutputPin as EmbeddedOutputPin};
use embedded_hal::i2c::{ErrorType as I2cErrorType, I2c as EmbeddedI2c, SevenBitAddress};
use platform_esp32::gpio::Esp32OutputPin;
use platform_esp32::i2c::Esp32I2c;

#[derive(Clone)]
struct DummyOutputPin {
    history: Rc<RefCell<Vec<bool>>>,
}

impl DummyOutputPin {
    fn new() -> Self {
        Self {
            history: Rc::new(RefCell::new(Vec::new())),
        }
    }
}

impl DigitalErrorType for DummyOutputPin {
    type Error = Infallible;
}

impl EmbeddedOutputPin for DummyOutputPin {
    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.history.borrow_mut().push(true);
        Ok(())
    }

    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.history.borrow_mut().push(false);
        Ok(())
    }
}

#[derive(Clone)]
struct DummyI2c {
    reads: Rc<RefCell<Vec<(u8, usize)>>>,
}

impl DummyI2c {
    fn new() -> Self {
        Self {
            reads: Rc::new(RefCell::new(Vec::new())),
        }
    }
}

impl I2cErrorType for DummyI2c {
    type Error = Infallible;
}

impl EmbeddedI2c<SevenBitAddress> for DummyI2c {
    fn transaction(
        &mut self,
        address: u8,
        operations: &mut [embedded_hal::i2c::Operation<'_>],
    ) -> Result<(), Self::Error> {
        for operation in operations {
            if let embedded_hal::i2c::Operation::Read(buffer) = operation {
                self.reads.borrow_mut().push((address, buffer.len()));
                buffer.fill(0xAB);
            }
        }

        Ok(())
    }
}

#[test]
fn core_app_runs_with_esp32_adapters() {
    let raw_pin = DummyOutputPin::new();
    let raw_i2c = DummyI2c::new();
    let pin_history = raw_pin.history.clone();
    let i2c_reads = raw_i2c.reads.clone();

    let pin = Esp32OutputPin::new(raw_pin);
    let i2c = Esp32I2c::new(raw_i2c);
    let mut app = App::new(pin, i2c);

    for _ in 0..500 {
        app.tick().unwrap();
    }

    assert_eq!(pin_history.borrow().as_slice(), &[true, false, true, false, true]);
    assert_eq!(i2c_reads.borrow().as_slice(), &[(0x48, 4)]);
}
