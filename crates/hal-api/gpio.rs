pub trait OutputPin {
    type Error;

    fn set_high(&mut self) -> Result<(), Self::Error>;
    fn set_low(&mut self) -> Result<(), Self::Error>;

    fn set(&mut self, high: bool) -> Result<(), Self::Error> {
        if high {
            self.set_high()
        } else {
            self.set_low()
        }
    }
}

pub trait InputPin {
    type Error;

    fn is_high(&self) -> Result<bool, Self::Error>;
    fn is_low(&self) -> Result<bool, Self::Error>;
}

