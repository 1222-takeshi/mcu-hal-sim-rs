//! ESP32 I2C アダプタ

use embedded_hal::i2c::{
    Error as EmbeddedI2cError,
    ErrorKind as EmbeddedI2cErrorKind,
    I2c as EmbeddedI2c,
    NoAcknowledgeSource,
    SevenBitAddress,
};
use hal_api::error::I2cError;
use hal_api::i2c::I2cBus;

fn map_i2c_error(error: impl EmbeddedI2cError) -> I2cError {
    match error.kind() {
        EmbeddedI2cErrorKind::NoAcknowledge(NoAcknowledgeSource::Address) => I2cError::InvalidAddress,
        EmbeddedI2cErrorKind::Bus
        | EmbeddedI2cErrorKind::ArbitrationLoss
        | EmbeddedI2cErrorKind::NoAcknowledge(NoAcknowledgeSource::Data)
        | EmbeddedI2cErrorKind::NoAcknowledge(NoAcknowledgeSource::Unknown)
        | EmbeddedI2cErrorKind::Overrun
        | EmbeddedI2cErrorKind::Other => I2cError::BusError,
        _ => I2cError::BusError,
    }
}

/// ESP32 向けの I2C バスラッパー。
///
/// `esp-hal` を含む `embedded-hal` v1.0 互換の I2C 実装を包み、
/// `hal-api::i2c::I2cBus` に接続します。
pub struct Esp32I2c<I> {
    inner: I,
}

impl<I> Esp32I2c<I> {
    /// ラップ対象の I2C 実装からアダプタを生成します。
    pub fn new(inner: I) -> Self {
        Self { inner }
    }

    /// 内部の I2C 実装参照を取得します。
    pub fn inner(&self) -> &I {
        &self.inner
    }

    /// 内部の I2C 実装可変参照を取得します。
    pub fn inner_mut(&mut self) -> &mut I {
        &mut self.inner
    }

    /// 内部の I2C 実装を取り出します。
    pub fn into_inner(self) -> I {
        self.inner
    }
}

impl<I> I2cBus for Esp32I2c<I>
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

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use core::convert::Infallible;

    use super::*;
    use embedded_hal::i2c::ErrorKind as EmbeddedI2cErrorKind;

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

    #[test]
    fn esp32_i2c_delegates_write_and_read() {
        let inner = DummyI2c {
            writes: 0,
            reads: 0,
            last_addr: None,
        };
        let mut i2c = Esp32I2c::new(inner);
        let mut buffer = [0u8; 4];

        i2c.write(0x48, &[0x01]).unwrap();
        i2c.read(0x48, &mut buffer).unwrap();

        assert_eq!(i2c.inner().writes, 1);
        assert_eq!(i2c.inner().reads, 1);
        assert_eq!(i2c.inner().last_addr, Some(0x48));
        assert_eq!(buffer, [0xAB; 4]);
    }

    #[test]
    fn esp32_i2c_delegates_write_read() {
        let inner = DummyI2c {
            writes: 0,
            reads: 0,
            last_addr: None,
        };
        let mut i2c = Esp32I2c::new(inner);
        let mut buffer = [0u8; 2];

        i2c.write_read(0x20, &[0x03], &mut buffer).unwrap();

        assert_eq!(i2c.inner().writes, 1);
        assert_eq!(i2c.inner().reads, 1);
        assert_eq!(i2c.inner().last_addr, Some(0x20));
        assert_eq!(buffer, [0xAB; 2]);
    }

    #[test]
    fn esp32_i2c_into_inner_returns_wrapped_bus() {
        let i2c = Esp32I2c::new(DummyI2c {
            writes: 0,
            reads: 0,
            last_addr: None,
        });

        assert_eq!(i2c.into_inner().writes, 0);
    }

    #[test]
    fn esp32_i2c_maps_address_errors() {
        let mut i2c = Esp32I2c::new(FailingI2c {
            error: DummyI2cDriverError(EmbeddedI2cErrorKind::NoAcknowledge(
                NoAcknowledgeSource::Address,
            )),
        });

        assert_eq!(i2c.write(0x48, &[0x01]), Err(I2cError::InvalidAddress));
    }

    #[test]
    fn esp32_i2c_maps_bus_errors() {
        let mut i2c = Esp32I2c::new(FailingI2c {
            error: DummyI2cDriverError(EmbeddedI2cErrorKind::Bus),
        });
        let mut buffer = [0u8; 2];

        assert_eq!(i2c.read(0x48, &mut buffer), Err(I2cError::BusError));
    }
}
