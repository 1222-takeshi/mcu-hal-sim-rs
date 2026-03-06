//! ESP32 GPIO アダプタ

use core::cell::{Ref, RefCell, RefMut};

use embedded_hal::digital::{InputPin as EmbeddedInputPin, OutputPin as EmbeddedOutputPin};
use hal_api::gpio::{InputPin, OutputPin};

/// ESP32 向けの出力ピンラッパー。
///
/// `esp-hal` を含む `embedded-hal` v1.0 互換の出力ピンを受け取り、
/// `hal-api::gpio::OutputPin` に橋渡しします。
pub struct Esp32OutputPin<P> {
    inner: P,
}

impl<P> Esp32OutputPin<P> {
    /// ラップ対象のピンからアダプタを生成します。
    pub fn new(inner: P) -> Self {
        Self { inner }
    }

    /// 内部のピン参照を取得します。
    pub fn inner(&self) -> &P {
        &self.inner
    }

    /// 内部の可変参照を取得します。
    pub fn inner_mut(&mut self) -> &mut P {
        &mut self.inner
    }

    /// 内部のピンを取り出します。
    pub fn into_inner(self) -> P {
        self.inner
    }
}

impl<P> OutputPin for Esp32OutputPin<P>
where
    P: EmbeddedOutputPin,
{
    type Error = P::Error;

    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.inner.set_high()
    }

    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.inner.set_low()
    }
}

/// ESP32 向けの入力ピンラッパー。
///
/// `embedded-hal` v1.0 の `InputPin` は `&mut self` を要求するため、
/// 内部では `RefCell` を使って可変借用を管理します。
pub struct Esp32InputPin<P> {
    inner: RefCell<P>,
}

impl<P> Esp32InputPin<P> {
    /// ラップ対象のピンからアダプタを生成します。
    pub fn new(inner: P) -> Self {
        Self {
            inner: RefCell::new(inner),
        }
    }

    /// 内部のピン参照を借用します。
    pub fn borrow_inner(&self) -> Ref<'_, P> {
        self.inner.borrow()
    }

    /// 内部のピン可変参照を借用します。
    pub fn borrow_inner_mut(&self) -> RefMut<'_, P> {
        self.inner.borrow_mut()
    }

    /// 内部のピンを取り出します。
    pub fn into_inner(self) -> P {
        self.inner.into_inner()
    }
}

impl<P> InputPin for Esp32InputPin<P>
where
    P: EmbeddedInputPin,
{
    type Error = P::Error;

    fn is_high(&self) -> Result<bool, Self::Error> {
        self.inner.borrow_mut().is_high()
    }

    fn is_low(&self) -> Result<bool, Self::Error> {
        self.inner.borrow_mut().is_low()
    }
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use core::convert::Infallible;

    use super::*;

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

    #[test]
    fn esp32_output_pin_delegates_to_inner_pin() {
        let inner = DummyOutputPin { level: false };
        let mut pin = Esp32OutputPin::new(inner);

        pin.set_high().unwrap();
        assert!(pin.inner().level);

        pin.set_low().unwrap();
        assert!(!pin.inner().level);
    }

    #[test]
    fn esp32_output_pin_into_inner_returns_wrapped_pin() {
        let inner = DummyOutputPin { level: true };
        let pin = Esp32OutputPin::new(inner);

        assert!(pin.into_inner().level);
    }

    #[test]
    fn esp32_input_pin_delegates_to_inner_pin() {
        let pin = Esp32InputPin::new(DummyInputPin {
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
    fn esp32_input_pin_into_inner_returns_wrapped_pin() {
        let pin = Esp32InputPin::new(DummyInputPin {
            level: false,
            high_reads: 0,
            low_reads: 0,
        });

        assert!(!pin.into_inner().level);
    }
}
