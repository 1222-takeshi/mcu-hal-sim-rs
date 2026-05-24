//! Generic `embedded-hal` v1.0 → HAL-API adapter types.
//!
//! `GenericOutputPin`, `GenericInputPin`, and `GenericI2c` are thin wrappers
//! around any `embedded-hal` v1.0 compatible peripheral.  Each platform crate
//! re-exports them under a platform-specific name (e.g. `Esp32OutputPin`,
//! `Rp2040OutputPin`, `AvrOutputPin`) to keep its own API surface stable.

use core::cell::{Ref, RefCell, RefMut};

use embedded_hal::digital::{
    Error as EmbeddedDigitalError, InputPin as EmbeddedInputPin, OutputPin as EmbeddedOutputPin,
};
use embedded_hal::i2c::{
    Error as EmbeddedI2cError, ErrorKind as EmbeddedI2cErrorKind, I2c as EmbeddedI2c,
    NoAcknowledgeSource, SevenBitAddress,
};

use crate::error::{GpioError, I2cError};
use crate::gpio::{InputPin, OutputPin};
use crate::i2c::I2cBus;

// ── Error mappers ──────────────────────────────────────────────────────────────

fn map_gpio_error(error: impl EmbeddedDigitalError) -> GpioError {
    // ErrorKind is #[non_exhaustive]; all variants map to HardwareError.
    let _ = error.kind();
    GpioError::HardwareError
}

fn map_i2c_error(error: impl EmbeddedI2cError) -> I2cError {
    match error.kind() {
        EmbeddedI2cErrorKind::NoAcknowledge(NoAcknowledgeSource::Address) => {
            I2cError::InvalidAddress
        }
        EmbeddedI2cErrorKind::Bus
        | EmbeddedI2cErrorKind::ArbitrationLoss
        | EmbeddedI2cErrorKind::NoAcknowledge(NoAcknowledgeSource::Data)
        | EmbeddedI2cErrorKind::NoAcknowledge(NoAcknowledgeSource::Unknown)
        | EmbeddedI2cErrorKind::Overrun
        | EmbeddedI2cErrorKind::Other => I2cError::BusError,
        // #[non_exhaustive] forward-compat: future variants default to BusError.
        _ => I2cError::BusError,
    }
}

// ── GenericOutputPin ───────────────────────────────────────────────────────────

/// Generic output-pin adapter for any `embedded-hal` v1.0 `OutputPin`.
pub struct GenericOutputPin<P> {
    inner: P,
}

impl<P> GenericOutputPin<P> {
    pub fn new(inner: P) -> Self {
        Self { inner }
    }

    pub fn inner(&self) -> &P {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut P {
        &mut self.inner
    }

    pub fn into_inner(self) -> P {
        self.inner
    }
}

impl<P> OutputPin for GenericOutputPin<P>
where
    P: EmbeddedOutputPin,
{
    type Error = GpioError;

    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.inner.set_high().map_err(map_gpio_error)
    }

    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.inner.set_low().map_err(map_gpio_error)
    }
}

// ── GenericInputPin ────────────────────────────────────────────────────────────

/// Generic input-pin adapter for any `embedded-hal` v1.0 `InputPin`.
///
/// `embedded-hal` v1.0 `InputPin::is_high` takes `&mut self`, so a `RefCell`
/// is used internally to allow `&self` reads as required by `hal-api`.
pub struct GenericInputPin<P> {
    inner: RefCell<P>,
}

impl<P> GenericInputPin<P> {
    pub fn new(inner: P) -> Self {
        Self {
            inner: RefCell::new(inner),
        }
    }

    pub fn borrow_inner(&self) -> Ref<'_, P> {
        self.inner.borrow()
    }

    pub fn borrow_inner_mut(&self) -> RefMut<'_, P> {
        self.inner.borrow_mut()
    }

    pub fn into_inner(self) -> P {
        self.inner.into_inner()
    }
}

impl<P> InputPin for GenericInputPin<P>
where
    P: EmbeddedInputPin,
{
    type Error = GpioError;

    fn is_high(&self) -> Result<bool, Self::Error> {
        self.inner.borrow_mut().is_high().map_err(map_gpio_error)
    }

    fn is_low(&self) -> Result<bool, Self::Error> {
        self.inner.borrow_mut().is_low().map_err(map_gpio_error)
    }
}

// ── GenericI2c ─────────────────────────────────────────────────────────────────

/// Generic I2C bus adapter for any `embedded-hal` v1.0 `I2c<SevenBitAddress>`.
pub struct GenericI2c<I> {
    inner: I,
}

impl<I> GenericI2c<I> {
    pub fn new(inner: I) -> Self {
        Self { inner }
    }

    pub fn inner(&self) -> &I {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut I {
        &mut self.inner
    }

    pub fn into_inner(self) -> I {
        self.inner
    }
}

impl<I> I2cBus for GenericI2c<I>
where
    I: EmbeddedI2c<SevenBitAddress>,
{
    type Error = I2cError;

    fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        self.inner.write(addr, bytes).map_err(map_i2c_error)
    }

    fn read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        self.inner.read(addr, buffer).map_err(map_i2c_error)
    }

    fn write_read(&mut self, addr: u8, bytes: &[u8], buffer: &mut [u8]) -> Result<(), Self::Error> {
        self.inner
            .write_read(addr, bytes, buffer)
            .map_err(map_i2c_error)
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use core::convert::Infallible;

    use super::*;
    use embedded_hal::digital::ErrorKind as EmbeddedDigitalErrorKind;
    use embedded_hal::i2c::ErrorKind as EmbeddedI2cErrorKind;

    // ── GPIO helpers ────────────────────────────────────────────────────────────

    struct DummyOutputPin {
        level: bool,
    }

    impl embedded_hal::digital::ErrorType for DummyOutputPin {
        type Error = Infallible;
    }

    impl EmbeddedOutputPin for DummyOutputPin {
        fn set_high(&mut self) -> Result<(), Self::Error> {
            self.level = true;
            Ok(())
        }

        fn set_low(&mut self) -> Result<(), Self::Error> {
            self.level = false;
            Ok(())
        }
    }

    #[derive(Debug, Clone, Copy)]
    struct DummyDigitalError;

    impl embedded_hal::digital::Error for DummyDigitalError {
        fn kind(&self) -> EmbeddedDigitalErrorKind {
            EmbeddedDigitalErrorKind::Other
        }
    }

    struct FailingOutputPin;

    impl embedded_hal::digital::ErrorType for FailingOutputPin {
        type Error = DummyDigitalError;
    }

    impl EmbeddedOutputPin for FailingOutputPin {
        fn set_high(&mut self) -> Result<(), Self::Error> {
            Err(DummyDigitalError)
        }

        fn set_low(&mut self) -> Result<(), Self::Error> {
            Err(DummyDigitalError)
        }
    }

    struct DummyInputPin {
        level: bool,
        high_reads: usize,
        low_reads: usize,
    }

    impl embedded_hal::digital::ErrorType for DummyInputPin {
        type Error = Infallible;
    }

    impl EmbeddedInputPin for DummyInputPin {
        fn is_high(&mut self) -> Result<bool, Self::Error> {
            self.high_reads += 1;
            Ok(self.level)
        }

        fn is_low(&mut self) -> Result<bool, Self::Error> {
            self.low_reads += 1;
            Ok(!self.level)
        }
    }

    struct FailingInputPin;

    impl embedded_hal::digital::ErrorType for FailingInputPin {
        type Error = DummyDigitalError;
    }

    impl EmbeddedInputPin for FailingInputPin {
        fn is_high(&mut self) -> Result<bool, Self::Error> {
            Err(DummyDigitalError)
        }

        fn is_low(&mut self) -> Result<bool, Self::Error> {
            Err(DummyDigitalError)
        }
    }

    // ── I2C helpers ─────────────────────────────────────────────────────────────

    struct DummyI2c {
        writes: usize,
        reads: usize,
        last_addr: Option<u8>,
    }

    impl embedded_hal::i2c::ErrorType for DummyI2c {
        type Error = Infallible;
    }

    impl EmbeddedI2c<SevenBitAddress> for DummyI2c {
        fn write(&mut self, addr: u8, _bytes: &[u8]) -> Result<(), Self::Error> {
            self.writes += 1;
            self.last_addr = Some(addr);
            Ok(())
        }

        fn read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
            self.reads += 1;
            self.last_addr = Some(addr);
            buffer.fill(0xAB);
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

        fn transaction(
            &mut self,
            addr: u8,
            operations: &mut [embedded_hal::i2c::Operation<'_>],
        ) -> Result<(), Self::Error> {
            self.last_addr = Some(addr);
            for operation in operations {
                match operation {
                    embedded_hal::i2c::Operation::Read(buffer) => {
                        self.reads += 1;
                        buffer.fill(0xAB);
                    }
                    embedded_hal::i2c::Operation::Write(bytes) => {
                        self.writes += 1;
                        let _ = bytes;
                    }
                }
            }
            Ok(())
        }
    }

    #[derive(Debug, Clone, Copy)]
    struct DummyI2cDriverError(EmbeddedI2cErrorKind);

    impl embedded_hal::i2c::Error for DummyI2cDriverError {
        fn kind(&self) -> EmbeddedI2cErrorKind {
            self.0
        }
    }

    struct FailingI2c {
        error: DummyI2cDriverError,
    }

    impl embedded_hal::i2c::ErrorType for FailingI2c {
        type Error = DummyI2cDriverError;
    }

    impl EmbeddedI2c<SevenBitAddress> for FailingI2c {
        fn transaction(
            &mut self,
            _addr: u8,
            _operations: &mut [embedded_hal::i2c::Operation<'_>],
        ) -> Result<(), Self::Error> {
            Err(self.error)
        }
    }

    // ── OutputPin tests ─────────────────────────────────────────────────────────

    #[test]
    fn output_pin_delegates_to_inner_pin() {
        let inner = DummyOutputPin { level: false };
        let mut pin = GenericOutputPin::new(inner);

        pin.set_high().unwrap();
        assert!(pin.inner().level);

        pin.set_low().unwrap();
        assert!(!pin.inner().level);
    }

    #[test]
    fn output_pin_into_inner_returns_wrapped_pin() {
        let inner = DummyOutputPin { level: true };
        let pin = GenericOutputPin::new(inner);

        assert!(pin.into_inner().level);
    }

    #[test]
    fn output_pin_maps_embedded_hal_errors() {
        let mut pin = GenericOutputPin::new(FailingOutputPin);

        assert_eq!(pin.set_high(), Err(GpioError::HardwareError));
        assert_eq!(pin.set_low(), Err(GpioError::HardwareError));
    }

    // ── InputPin tests ──────────────────────────────────────────────────────────

    #[test]
    fn input_pin_delegates_to_inner_pin() {
        let pin = GenericInputPin::new(DummyInputPin {
            level: true,
            high_reads: 0,
            low_reads: 0,
        });

        assert!(pin.is_high().unwrap());
        assert!(!pin.is_low().unwrap());
        assert_eq!(pin.borrow_inner().high_reads, 1);
        assert_eq!(pin.borrow_inner().low_reads, 1);
    }

    #[test]
    fn input_pin_borrow_inner_mut_returns_mutable_ref() {
        let pin = GenericInputPin::new(DummyInputPin {
            level: false,
            high_reads: 0,
            low_reads: 0,
        });
        pin.borrow_inner_mut().high_reads = 42;
        assert_eq!(pin.borrow_inner().high_reads, 42);
    }

    #[test]
    fn input_pin_into_inner_returns_wrapped_pin() {
        let pin = GenericInputPin::new(DummyInputPin {
            level: false,
            high_reads: 0,
            low_reads: 0,
        });

        assert!(!pin.into_inner().level);
    }

    #[test]
    fn input_pin_maps_embedded_hal_errors() {
        let pin = GenericInputPin::new(FailingInputPin);

        assert_eq!(pin.is_high(), Err(GpioError::HardwareError));
        assert_eq!(pin.is_low(), Err(GpioError::HardwareError));
    }

    // ── I2c tests ───────────────────────────────────────────────────────────────

    #[test]
    fn i2c_delegates_write_and_read() {
        let inner = DummyI2c {
            writes: 0,
            reads: 0,
            last_addr: None,
        };
        let mut i2c = GenericI2c::new(inner);
        let mut buffer = [0u8; 4];

        i2c.write(0x48, &[0x01]).unwrap();
        i2c.read(0x48, &mut buffer).unwrap();

        assert_eq!(i2c.inner().writes, 1);
        assert_eq!(i2c.inner().reads, 1);
        assert_eq!(i2c.inner().last_addr, Some(0x48));
        assert_eq!(buffer, [0xAB; 4]);
    }

    #[test]
    fn i2c_delegates_write_read() {
        let inner = DummyI2c {
            writes: 0,
            reads: 0,
            last_addr: None,
        };
        let mut i2c = GenericI2c::new(inner);
        let mut buffer = [0u8; 2];

        i2c.write_read(0x20, &[0x03], &mut buffer).unwrap();

        assert_eq!(i2c.inner().writes, 1);
        assert_eq!(i2c.inner().reads, 1);
        assert_eq!(i2c.inner().last_addr, Some(0x20));
        assert_eq!(buffer, [0xAB; 2]);
    }

    #[test]
    fn i2c_into_inner_returns_wrapped_bus() {
        let i2c = GenericI2c::new(DummyI2c {
            writes: 0,
            reads: 0,
            last_addr: None,
        });

        assert_eq!(i2c.into_inner().writes, 0);
    }

    #[test]
    fn i2c_maps_address_errors() {
        let mut i2c = GenericI2c::new(FailingI2c {
            error: DummyI2cDriverError(EmbeddedI2cErrorKind::NoAcknowledge(
                NoAcknowledgeSource::Address,
            )),
        });

        assert_eq!(i2c.write(0x48, &[0x01]), Err(I2cError::InvalidAddress));
    }

    #[test]
    fn i2c_maps_bus_errors() {
        let mut i2c = GenericI2c::new(FailingI2c {
            error: DummyI2cDriverError(EmbeddedI2cErrorKind::Bus),
        });
        let mut buffer = [0u8; 2];

        assert_eq!(i2c.read(0x48, &mut buffer), Err(I2cError::BusError));
    }

    #[test]
    fn i2c_maps_write_read_errors() {
        let mut i2c = GenericI2c::new(FailingI2c {
            error: DummyI2cDriverError(EmbeddedI2cErrorKind::ArbitrationLoss),
        });
        let mut buffer = [0u8; 2];

        assert_eq!(
            i2c.write_read(0x48, &[0x01], &mut buffer),
            Err(I2cError::BusError)
        );
    }
}
