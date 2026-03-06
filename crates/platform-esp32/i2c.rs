//! ESP32 I2C アダプタ

use hal_api::i2c::I2cBus;

/// ESP32 向けの I2C バスラッパー。
///
/// 将来的には `esp-hal` の I2C 実装を包み、`hal-api::i2c::I2cBus`
/// に接続する責務を持たせます。
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
    I: I2cBus,
{
    type Error = I::Error;

    fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        self.inner.write(addr, bytes)
    }

    fn read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        self.inner.read(addr, buffer)
    }

    fn write_read(&mut self, addr: u8, bytes: &[u8], buffer: &mut [u8]) -> Result<(), Self::Error> {
        self.inner.write_read(addr, bytes, buffer)
    }
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;
    use hal_api::error::I2cError;

    struct DummyI2c {
        writes: usize,
        reads: usize,
        last_addr: Option<u8>,
    }

    impl I2cBus for DummyI2c {
        type Error = I2cError;

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
}
