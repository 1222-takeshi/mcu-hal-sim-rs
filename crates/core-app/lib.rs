use hal_api::error::{GpioError, I2cError};
use hal_api::gpio::OutputPin;
use hal_api::i2c::I2cBus;

#[derive(Debug)]
pub enum AppError {
    Gpio(GpioError),
    I2c(I2cError),
}

pub struct App<PIN, I2C> {
    pin: PIN,
    i2c: I2C,
    tick_count: u32,
    led_state: bool,
}

impl<PIN, I2C> App<PIN, I2C>
where
    PIN: OutputPin,
    I2C: I2cBus,
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

        // 1秒ごと（100 tick想定）にLED切り替え
        if self.tick_count % 100 == 0 {
            self.led_state = !self.led_state;
            self.pin.set(self.led_state).map_err(AppError::Gpio)?;
        }

        // 5秒ごとにダミーI2C読み取り
        if self.tick_count % 500 == 0 {
            let mut buffer = [0u8; 4];
            self.i2c.read(0x48, &mut buffer).map_err(AppError::I2c)?;
        }

        Ok(())
    }
}
