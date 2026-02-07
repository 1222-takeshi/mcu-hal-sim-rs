use hal_api::error::{GpioError, I2cError};
use hal_api::gpio::OutputPin;
use hal_api::i2c::I2cBus;

#[derive(Debug)]
pub enum AppError {
    Gpio(GpioError),
    I2c(I2cError),
}

impl From<GpioError> for AppError {
    fn from(err: GpioError) -> Self {
        AppError::Gpio(err)
    }
}

impl From<I2cError> for AppError {
    fn from(err: I2cError) -> Self {
        AppError::I2c(err)
    }
}

pub struct App<PIN, I2C> {
    pin: PIN,
    i2c: I2C,
    tick_count: u32,
    led_state: bool,
}

impl<PIN, I2C> App<PIN, I2C>
where
    PIN: OutputPin<Error = GpioError>,
    I2C: I2cBus<Error = I2cError>,
{
    pub fn new(pin: PIN, i2c: I2C) -> Self {
        Self {
            pin,
            i2c,
            tick_count: 0,
            led_state: false,
        }
    }

    pub fn tick(&mut self) -> Result<(), AppError> {
        self.tick_count += 1;

        // 100 tickごと（1秒想定）にLED切り替え
        if self.tick_count % 100 == 0 {
            self.led_state = !self.led_state;
            self.pin.set(self.led_state)?;
        }

        // 500 tickごと（5秒想定）にI2C読み取り
        if self.tick_count % 500 == 0 {
            let mut buffer = [0u8; 4];
            self.i2c.read(0x48, &mut buffer)?;
        }

        Ok(())
    }
}

