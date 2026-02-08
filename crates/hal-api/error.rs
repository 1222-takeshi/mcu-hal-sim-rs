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


