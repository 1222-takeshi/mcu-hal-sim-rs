//! ESP32 GPIO アダプタ

use hal_api::gpio::{InputPin, OutputPin};

/// ESP32 向けの出力ピンラッパー。
///
/// 今は具体的な ESP32 HAL 型に依存せず、後段で `esp-hal` のピン型を
/// 包めるように最小限の薄いラッパーとして定義しています。
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
    P: OutputPin,
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
pub struct Esp32InputPin<P> {
    inner: P,
}

impl<P> Esp32InputPin<P> {
    /// ラップ対象のピンからアダプタを生成します。
    pub fn new(inner: P) -> Self {
        Self { inner }
    }

    /// 内部のピン参照を取得します。
    pub fn inner(&self) -> &P {
        &self.inner
    }

    /// 内部のピンを取り出します。
    pub fn into_inner(self) -> P {
        self.inner
    }
}

impl<P> InputPin for Esp32InputPin<P>
where
    P: InputPin,
{
    type Error = P::Error;

    fn is_high(&self) -> Result<bool, Self::Error> {
        self.inner.is_high()
    }

    fn is_low(&self) -> Result<bool, Self::Error> {
        self.inner.is_low()
    }
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;
    use hal_api::error::GpioError;

    struct DummyOutputPin {
        level: bool,
    }

    impl OutputPin for DummyOutputPin {
        type Error = GpioError;

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
    }

    impl InputPin for DummyInputPin {
        type Error = GpioError;

        fn is_high(&self) -> Result<bool, Self::Error> {
            Ok(self.level)
        }

        fn is_low(&self) -> Result<bool, Self::Error> {
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
        let pin = Esp32InputPin::new(DummyInputPin { level: true });

        assert!(pin.is_high().unwrap());
        assert!(!pin.is_low().unwrap());
    }

    #[test]
    fn esp32_input_pin_into_inner_returns_wrapped_pin() {
        let pin = Esp32InputPin::new(DummyInputPin { level: false });

        assert!(!pin.into_inner().level);
    }
}
