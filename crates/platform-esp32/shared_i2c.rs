//! 共有 I2C バスアダプタ

use core::cell::RefCell;

use hal_api::error::I2cError;
use hal_api::i2c::I2cBus;

/// `RefCell` 上の I2C バスを複数デバイスへ共有するための薄いラッパーです。
#[derive(Clone, Copy)]
pub struct SharedI2cBus<'a, B> {
    inner: &'a RefCell<B>,
}

impl<'a, B> SharedI2cBus<'a, B> {
    pub const fn new(inner: &'a RefCell<B>) -> Self {
        Self { inner }
    }
}

impl<B> I2cBus for SharedI2cBus<'_, B>
where
    B: I2cBus<Error = I2cError>,
{
    type Error = I2cError;

    fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        self.inner.borrow_mut().write(addr, bytes)
    }

    fn read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        self.inner.borrow_mut().read(addr, buffer)
    }

    fn write_read(&mut self, addr: u8, bytes: &[u8], buffer: &mut [u8]) -> Result<(), Self::Error> {
        self.inner.borrow_mut().write_read(addr, bytes, buffer)
    }
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::RefCell;
    use std::rc::Rc;
    use std::vec;
    use std::vec::Vec;

    type WriteLog = Rc<RefCell<Vec<(u8, Vec<u8>)>>>;

    #[derive(Default)]
    struct DummyI2c {
        writes: WriteLog,
    }

    impl I2cBus for DummyI2c {
        type Error = I2cError;

        fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
            self.writes.borrow_mut().push((addr, bytes.to_vec()));
            Ok(())
        }

        fn read(&mut self, _addr: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
            buffer.fill(0xAA);
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
    fn shared_i2c_bus_allows_multiple_handles() {
        let inner = RefCell::new(DummyI2c::default());
        let writes = inner.borrow().writes.clone();
        let mut bus_a = SharedI2cBus::new(&inner);
        let mut bus_b = SharedI2cBus::new(&inner);
        let mut buffer = [0u8; 2];

        bus_a.write(0x27, &[0x01]).unwrap();
        bus_b.write_read(0x77, &[0xF7], &mut buffer).unwrap();

        assert_eq!(
            writes.borrow().as_slice(),
            &[(0x27, vec![0x01]), (0x77, vec![0xF7])]
        );
        assert_eq!(buffer, [0xAA, 0xAA]);
    }
}
