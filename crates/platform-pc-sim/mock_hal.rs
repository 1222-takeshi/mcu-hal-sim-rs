use hal_api::error::{GpioError, I2cError};
use hal_api::gpio::OutputPin;
use hal_api::i2c::I2cBus;

pub struct MockPin {
    pin_number: u8,
    state: bool,
}

impl MockPin {
    pub fn new(pin_number: u8) -> Self {
        Self {
            pin_number,
            state: false,
        }
    }
}

impl OutputPin for MockPin {
    type Error = GpioError;

    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.state = true;
        println!("[GPIO] Pin {} set HIGH", self.pin_number);
        Ok(())
    }

    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.state = false;
        println!("[GPIO] Pin {} set LOW", self.pin_number);
        Ok(())
    }
}

pub struct MockI2c;

impl MockI2c {
    pub fn new() -> Self {
        Self
    }
}

impl I2cBus for MockI2c {
    type Error = I2cError;

    fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        println!("[I2C] Write to 0x{:02X}: {:?}", addr, bytes);
        Ok(())
    }

    fn read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        println!("[I2C] Read from 0x{:02X}: {} bytes", addr, buffer.len());
        buffer.fill(0xFF);
        Ok(())
    }

    fn write_read(
        &mut self,
        addr: u8,
        bytes: &[u8],
        buffer: &mut [u8],
    ) -> Result<(), Self::Error> {
        self.write(addr, bytes)?;
        self.read(addr, buffer)
    }
}
