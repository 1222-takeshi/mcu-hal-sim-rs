use hal-api::gpio::OutputPin;
use hal-api::i2c::I2cBus;

pub struct App<PIN, I2C> {
    pin: PIN,
    i2c: I2C,
}

impl<PIN, I2C> App<PIN, I2C>
where
    PIN: OutputPin,
    I2C: I2cBus,
{
    pub fn new(pin: PIN, i2c: I2C) -> Self {
        Self { pin, i2c }
    }

    pub fn tick(&mut self) {
        // TODO: implement application logic
        let _ = &mut self.pin;
        let _ = &mut self.i2c;
    }
}

