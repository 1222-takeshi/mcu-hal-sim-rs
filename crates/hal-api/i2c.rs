pub trait I2cBus {
    type Error;

    fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error>;
    fn read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), Self::Error>;
    fn write_read(
        &mut self,
        addr: u8,
        bytes: &[u8],
        buffer: &mut [u8],
    ) -> Result<(), Self::Error>;
}

