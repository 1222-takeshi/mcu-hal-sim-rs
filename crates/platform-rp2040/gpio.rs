//! RP2040 GPIO アダプタ

use core::cell::{Ref, RefCell};

use embedded_hal::digital::{
    Error as EmbeddedDigitalError, InputPin as EmbeddedInputPin, OutputPin as EmbeddedOutputPin,
};
use hal_api::error::GpioError;
use hal_api::gpio::{InputPin, OutputPin};

fn map_gpio_error(error: impl EmbeddedDigitalError) -> GpioError {
    // ErrorKind is #[non_exhaustive]; all variants map to HardwareError.
    let _ = error.kind();
    GpioError::HardwareError
}

/// RP2040 向けの出力ピンラッパー。
pub struct Rp2040OutputPin<P> {
    inner: P,
}

impl<P> Rp2040OutputPin<P> {
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

impl<P> OutputPin for Rp2040OutputPin<P>
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

/// RP2040 向けの入力ピンラッパー。
pub struct Rp2040InputPin<P> {
    inner: RefCell<P>,
}

impl<P> Rp2040InputPin<P> {
    pub fn new(inner: P) -> Self {
        Self {
            inner: RefCell::new(inner),
        }
    }

    pub fn borrow_inner(&self) -> Ref<'_, P> {
        self.inner.borrow()
    }

    pub fn into_inner(self) -> P {
        self.inner.into_inner()
    }
}

impl<P> InputPin for Rp2040InputPin<P>
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

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use core::convert::Infallible;

    use super::*;
    use embedded_hal::digital::ErrorKind as EmbeddedDigitalErrorKind;

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

    #[test]
    fn rp2040_output_pin_delegates_to_inner_pin() {
        let inner = DummyOutputPin { level: false };
        let mut pin = Rp2040OutputPin::new(inner);

        pin.set_high().unwrap();
        assert!(pin.inner().level);

        pin.set_low().unwrap();
        assert!(!pin.inner().level);
    }

    #[test]
    fn rp2040_output_pin_maps_embedded_hal_errors() {
        let mut pin = Rp2040OutputPin::new(FailingOutputPin);

        assert_eq!(pin.set_high(), Err(GpioError::HardwareError));
        assert_eq!(pin.set_low(), Err(GpioError::HardwareError));
    }

    #[test]
    fn rp2040_input_pin_delegates_to_inner_pin() {
        let pin = Rp2040InputPin::new(DummyInputPin {
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
    fn rp2040_input_pin_maps_embedded_hal_errors() {
        let pin = Rp2040InputPin::new(FailingInputPin);

        assert_eq!(pin.is_high(), Err(GpioError::HardwareError));
        assert_eq!(pin.is_low(), Err(GpioError::HardwareError));
    }
}
