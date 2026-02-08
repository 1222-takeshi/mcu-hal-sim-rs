//! GPIO (General Purpose Input/Output) HAL trait定義

/// 出力ピンを制御するためのtrait
///
/// このtraitは、GPIO出力ピンの基本的な操作を定義します。
///
/// # Examples
///
/// ```
/// use hal_api::gpio::OutputPin;
/// use hal_api::error::GpioError;
///
/// struct MockPin {
///     state: bool,
/// }
///
/// impl OutputPin for MockPin {
///     type Error = GpioError;
///
///     fn set_high(&mut self) -> Result<(), Self::Error> {
///         self.state = true;
///         Ok(())
///     }
///
///     fn set_low(&mut self) -> Result<(), Self::Error> {
///         self.state = false;
///         Ok(())
///     }
/// }
///
/// let mut pin = MockPin { state: false };
/// pin.set_high().unwrap();
/// assert_eq!(pin.state, true);
/// ```
pub trait OutputPin {
    /// エラー型
    type Error;

    /// ピンをHIGH（1）に設定
    ///
    /// # Examples
    ///
    /// ```
    /// use hal_api::gpio::OutputPin;
    /// use hal_api::error::GpioError;
    ///
    /// struct TestPin;
    ///
    /// impl OutputPin for TestPin {
    ///     type Error = GpioError;
    ///     fn set_high(&mut self) -> Result<(), Self::Error> { Ok(()) }
    ///     fn set_low(&mut self) -> Result<(), Self::Error> { Ok(()) }
    /// }
    ///
    /// let mut pin = TestPin;
    /// assert!(pin.set_high().is_ok());
    /// ```
    fn set_high(&mut self) -> Result<(), Self::Error>;

    /// ピンをLOW（0）に設定
    ///
    /// # Examples
    ///
    /// ```
    /// use hal_api::gpio::OutputPin;
    /// use hal_api::error::GpioError;
    ///
    /// struct TestPin;
    ///
    /// impl OutputPin for TestPin {
    ///     type Error = GpioError;
    ///     fn set_high(&mut self) -> Result<(), Self::Error> { Ok(()) }
    ///     fn set_low(&mut self) -> Result<(), Self::Error> { Ok(()) }
    /// }
    ///
    /// let mut pin = TestPin;
    /// assert!(pin.set_low().is_ok());
    /// ```
    fn set_low(&mut self) -> Result<(), Self::Error>;

    /// ピンを指定された状態に設定
    ///
    /// # Arguments
    ///
    /// * `high` - `true`ならHIGH、`false`ならLOW
    ///
    /// # Examples
    ///
    /// ```
    /// use hal_api::gpio::OutputPin;
    /// use hal_api::error::GpioError;
    ///
    /// struct TestPin { state: bool }
    ///
    /// impl OutputPin for TestPin {
    ///     type Error = GpioError;
    ///     fn set_high(&mut self) -> Result<(), Self::Error> {
    ///         self.state = true;
    ///         Ok(())
    ///     }
    ///     fn set_low(&mut self) -> Result<(), Self::Error> {
    ///         self.state = false;
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let mut pin = TestPin { state: false };
    /// pin.set(true).unwrap();
    /// assert_eq!(pin.state, true);
    /// pin.set(false).unwrap();
    /// assert_eq!(pin.state, false);
    /// ```
    fn set(&mut self, high: bool) -> Result<(), Self::Error> {
        if high {
            self.set_high()
        } else {
            self.set_low()
        }
    }
}

/// 入力ピンから値を読み取るためのtrait
///
/// # Examples
///
/// ```
/// use hal_api::gpio::InputPin;
/// use hal_api::error::GpioError;
///
/// struct MockInputPin {
///     state: bool,
/// }
///
/// impl InputPin for MockInputPin {
///     type Error = GpioError;
///
///     fn is_high(&self) -> Result<bool, Self::Error> {
///         Ok(self.state)
///     }
///
///     fn is_low(&self) -> Result<bool, Self::Error> {
///         Ok(!self.state)
///     }
/// }
///
/// let pin = MockInputPin { state: true };
/// assert_eq!(pin.is_high().unwrap(), true);
/// assert_eq!(pin.is_low().unwrap(), false);
/// ```
pub trait InputPin {
    /// エラー型
    type Error;

    /// ピンがHIGH（1）かどうかを確認
    fn is_high(&self) -> Result<bool, Self::Error>;

    /// ピンがLOW（0）かどうかを確認
    fn is_low(&self) -> Result<bool, Self::Error>;
}
