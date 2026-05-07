//! エラー型定義
//!
//! HAL操作で発生する可能性のあるエラーを定義します。

/// GPIO操作に関連するエラー
///
/// # Examples
///
/// ```
/// use hal_api::error::GpioError;
///
/// let error = GpioError::InvalidPin;
/// assert_eq!(format!("{:?}", error), "InvalidPin");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpioError {
    /// ピン番号が無効
    ///
    /// # Examples
    ///
    /// ```
    /// use hal_api::error::GpioError;
    ///
    /// let error = GpioError::InvalidPin;
    /// assert!(matches!(error, GpioError::InvalidPin));
    /// ```
    InvalidPin,

    /// ハードウェアエラー
    ///
    /// # Examples
    ///
    /// ```
    /// use hal_api::error::GpioError;
    ///
    /// let error = GpioError::HardwareError;
    /// assert!(matches!(error, GpioError::HardwareError));
    /// ```
    HardwareError,
}

/// I2C操作に関連するエラー
///
/// # Examples
///
/// ```
/// use hal_api::error::I2cError;
///
/// let error = I2cError::BusError;
/// assert_eq!(format!("{:?}", error), "BusError");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum I2cError {
    /// I2Cアドレスが無効
    ///
    /// # Examples
    ///
    /// ```
    /// use hal_api::error::I2cError;
    ///
    /// let error = I2cError::InvalidAddress;
    /// assert!(matches!(error, I2cError::InvalidAddress));
    /// ```
    InvalidAddress,

    /// バスエラー
    ///
    /// # Examples
    ///
    /// ```
    /// use hal_api::error::I2cError;
    ///
    /// let error = I2cError::BusError;
    /// assert!(matches!(error, I2cError::BusError));
    /// ```
    BusError,

    /// タイムアウト
    ///
    /// # Examples
    ///
    /// ```
    /// use hal_api::error::I2cError;
    ///
    /// let error = I2cError::Timeout;
    /// assert!(matches!(error, I2cError::Timeout));
    /// ```
    Timeout,
}

/// センサ読み取りに関連するエラー
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SensorError {
    /// 通信または下位層のバスエラー
    BusError,
    /// センサが測定中で結果がまだ確定していない
    Busy,
    /// 取得した値が不正
    InvalidReading,
    /// 初期化未完了
    NotInitialized,
}

/// 文字表示デバイスに関連するエラー
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisplayError {
    /// 通信または下位層のバスエラー
    BusError,
    /// 表示内容が不正
    InvalidContent,
    /// 初期化未完了
    NotInitialized,
}

/// サーボやモータ出力に関連するエラー
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActuatorError {
    /// 指令値が範囲外
    InvalidCommand,
    /// 通信または下位層のハードウェアエラー
    HardwareError,
}

impl From<GpioError> for ActuatorError {
    fn from(_: GpioError) -> Self {
        ActuatorError::HardwareError
    }
}

#[cfg(feature = "std")]
mod std_impls {
    use super::{ActuatorError, DisplayError, GpioError, I2cError, SensorError};

    impl std::error::Error for GpioError {}
    impl std::error::Error for I2cError {}
    impl std::error::Error for SensorError {}
    impl std::error::Error for DisplayError {}
    impl std::error::Error for ActuatorError {}
}

#[cfg(feature = "std")]
impl std::fmt::Display for GpioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GpioError::InvalidPin => write!(f, "invalid GPIO pin"),
            GpioError::HardwareError => write!(f, "GPIO hardware error"),
        }
    }
}

#[cfg(feature = "std")]
impl std::fmt::Display for I2cError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            I2cError::InvalidAddress => write!(f, "invalid I2C address"),
            I2cError::BusError => write!(f, "I2C bus error"),
            I2cError::Timeout => write!(f, "I2C timeout"),
        }
    }
}

#[cfg(feature = "std")]
impl std::fmt::Display for SensorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SensorError::BusError => write!(f, "sensor bus error"),
            SensorError::Busy => write!(f, "sensor busy"),
            SensorError::InvalidReading => write!(f, "invalid sensor reading"),
            SensorError::NotInitialized => write!(f, "sensor not initialized"),
        }
    }
}

#[cfg(feature = "std")]
impl std::fmt::Display for DisplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DisplayError::BusError => write!(f, "display bus error"),
            DisplayError::InvalidContent => write!(f, "invalid display content"),
            DisplayError::NotInitialized => write!(f, "display not initialized"),
        }
    }
}

#[cfg(feature = "std")]
impl std::fmt::Display for ActuatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActuatorError::InvalidCommand => write!(f, "invalid actuator command"),
            ActuatorError::HardwareError => write!(f, "actuator hardware error"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpio_error_debug() {
        assert_eq!(format!("{:?}", GpioError::InvalidPin), "InvalidPin");
        assert_eq!(format!("{:?}", GpioError::HardwareError), "HardwareError");
    }

    #[test]
    fn i2c_error_debug() {
        assert_eq!(format!("{:?}", I2cError::BusError), "BusError");
        assert_eq!(format!("{:?}", I2cError::Timeout), "Timeout");
        assert_eq!(format!("{:?}", I2cError::InvalidAddress), "InvalidAddress");
    }

    #[cfg(feature = "std")]
    #[test]
    fn gpio_error_display() {
        assert_eq!(GpioError::InvalidPin.to_string(), "invalid GPIO pin");
        assert_eq!(GpioError::HardwareError.to_string(), "GPIO hardware error");
    }

    #[cfg(feature = "std")]
    #[test]
    fn i2c_error_display() {
        assert_eq!(I2cError::BusError.to_string(), "I2C bus error");
        assert_eq!(I2cError::Timeout.to_string(), "I2C timeout");
        assert_eq!(I2cError::InvalidAddress.to_string(), "invalid I2C address");
    }

    #[cfg(feature = "std")]
    #[test]
    fn sensor_error_display() {
        assert_eq!(SensorError::BusError.to_string(), "sensor bus error");
        assert_eq!(SensorError::Busy.to_string(), "sensor busy");
        assert_eq!(
            SensorError::InvalidReading.to_string(),
            "invalid sensor reading"
        );
        assert_eq!(
            SensorError::NotInitialized.to_string(),
            "sensor not initialized"
        );
    }

    #[cfg(feature = "std")]
    #[test]
    fn display_error_display() {
        assert_eq!(DisplayError::BusError.to_string(), "display bus error");
        assert_eq!(
            DisplayError::InvalidContent.to_string(),
            "invalid display content"
        );
        assert_eq!(
            DisplayError::NotInitialized.to_string(),
            "display not initialized"
        );
    }

    #[cfg(feature = "std")]
    #[test]
    fn actuator_error_display() {
        assert_eq!(
            ActuatorError::InvalidCommand.to_string(),
            "invalid actuator command"
        );
        assert_eq!(
            ActuatorError::HardwareError.to_string(),
            "actuator hardware error"
        );
    }

    #[cfg(feature = "std")]
    #[test]
    fn errors_implement_std_error() {
        fn assert_error<E: std::error::Error>() {}
        assert_error::<GpioError>();
        assert_error::<I2cError>();
        assert_error::<SensorError>();
        assert_error::<DisplayError>();
        assert_error::<ActuatorError>();
    }

    #[test]
    fn actuator_from_gpio_error() {
        let gpio_err = GpioError::HardwareError;
        let act_err: ActuatorError = gpio_err.into();
        assert_eq!(act_err, ActuatorError::HardwareError);
    }
}
