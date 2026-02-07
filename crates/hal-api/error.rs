#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpioError {
    InvalidPin,
    HardwareError,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum I2cError {
    InvalidAddress,
    BusError,
    Timeout,
}
