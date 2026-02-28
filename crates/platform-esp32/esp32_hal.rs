//! ESP32向けHALアダプタ
//!
//! `embedded-hal`実装を`hal-api` traitへ変換するための薄いアダプタ層です。

use embedded_hal::digital::OutputPin as EmbeddedOutputPin;
use embedded_hal::i2c::I2c as EmbeddedI2c;
use hal_api::error::{GpioError, I2cError};
use hal_api::gpio::OutputPin;
use hal_api::i2c::I2cBus;

/// `embedded-hal`のOutputPinを`hal-api::OutputPin`へ適合させるアダプタ。
pub struct Esp32OutputPin<PIN> {
    pin: PIN,
}

impl<PIN> Esp32OutputPin<PIN> {
    #[cfg(any(test, feature = "esp32c3"))]
    pub fn new(pin: PIN) -> Self {
        Self { pin }
    }
}

impl<PIN> OutputPin for Esp32OutputPin<PIN>
where
    PIN: EmbeddedOutputPin,
{
    type Error = GpioError;

    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.pin.set_high().map_err(|_| GpioError::HardwareError)
    }

    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.pin.set_low().map_err(|_| GpioError::HardwareError)
    }
}

/// `embedded-hal`のI2Cを`hal-api::I2cBus`へ適合させるアダプタ。
pub struct Esp32I2c<I2C> {
    i2c: I2C,
}

impl<I2C> Esp32I2c<I2C> {
    #[cfg(any(test, feature = "esp32c3"))]
    pub fn new(i2c: I2C) -> Self {
        Self { i2c }
    }

    fn validate_addr(addr: u8) -> Result<(), I2cError> {
        if addr <= 0x7F {
            Ok(())
        } else {
            Err(I2cError::InvalidAddress)
        }
    }
}

impl<I2C> I2cBus for Esp32I2c<I2C>
where
    I2C: EmbeddedI2c,
{
    type Error = I2cError;

    fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        Self::validate_addr(addr)?;
        self.i2c.write(addr, bytes).map_err(|_| I2cError::BusError)
    }

    fn read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        Self::validate_addr(addr)?;
        self.i2c.read(addr, buffer).map_err(|_| I2cError::BusError)
    }

    fn write_read(&mut self, addr: u8, bytes: &[u8], buffer: &mut [u8]) -> Result<(), Self::Error> {
        Self::validate_addr(addr)?;
        self.i2c
            .write_read(addr, bytes, buffer)
            .map_err(|_| I2cError::BusError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::convert::Infallible;
    use embedded_hal::digital::ErrorType as DigitalErrorType;
    use embedded_hal::i2c::ErrorType as I2cErrorType;
    use embedded_hal::i2c::Operation;

    #[derive(Default)]
    struct TestPin {
        is_high: bool,
    }

    impl DigitalErrorType for TestPin {
        type Error = Infallible;
    }

    impl EmbeddedOutputPin for TestPin {
        fn set_low(&mut self) -> Result<(), Self::Error> {
            self.is_high = false;
            Ok(())
        }

        fn set_high(&mut self) -> Result<(), Self::Error> {
            self.is_high = true;
            Ok(())
        }
    }

    #[derive(Default)]
    struct TestI2c {
        read_count: usize,
    }

    impl I2cErrorType for TestI2c {
        type Error = Infallible;
    }

    impl EmbeddedI2c for TestI2c {
        fn transaction(
            &mut self,
            _address: u8,
            operations: &mut [Operation<'_>],
        ) -> Result<(), Self::Error> {
            for op in operations {
                if let Operation::Read(buf) = op {
                    self.read_count += 1;
                    buf.fill(0xAB);
                }
            }
            Ok(())
        }
    }

    #[test]
    fn test_esp32_output_pin_adapter() {
        let pin = TestPin::default();
        let mut adapter = Esp32OutputPin::new(pin);
        assert!(adapter.set_high().is_ok());
        assert!(adapter.set_low().is_ok());
    }

    #[test]
    fn test_esp32_i2c_adapter_read() {
        let i2c = TestI2c::default();
        let mut adapter = Esp32I2c::new(i2c);

        let mut buf = [0_u8; 4];
        assert!(adapter.read(0x48, &mut buf).is_ok());
        assert_eq!(buf, [0xAB; 4]);
    }

    #[test]
    fn test_esp32_i2c_adapter_rejects_invalid_address() {
        let i2c = TestI2c::default();
        let mut adapter = Esp32I2c::new(i2c);
        let mut buf = [0_u8; 2];

        let err = adapter.read(0x80, &mut buf).unwrap_err();
        assert_eq!(err, I2cError::InvalidAddress);
    }
}
