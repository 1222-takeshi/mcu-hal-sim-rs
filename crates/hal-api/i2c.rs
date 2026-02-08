//! I2C (Inter-Integrated Circuit) バスHAL trait定義

/// I2Cバス通信を行うためのtrait
///
/// このtraitは、I2Cバスの基本的な操作（書き込み、読み取り、書き込み後読み取り）を定義します。
///
/// # Examples
///
/// ```
/// use hal_api::i2c::I2cBus;
/// use hal_api::error::I2cError;
///
/// struct MockI2c;
///
/// impl I2cBus for MockI2c {
///     type Error = I2cError;
///
///     fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
///         Ok(())
///     }
///
///     fn read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
///         buffer.fill(0);
///         Ok(())
///     }
///
///     fn write_read(
///         &mut self,
///         addr: u8,
///         bytes: &[u8],
///         buffer: &mut [u8],
///     ) -> Result<(), Self::Error> {
///         buffer.fill(0);
///         Ok(())
///     }
/// }
///
/// let mut i2c = MockI2c;
/// assert!(i2c.write(0x48, &[0x01, 0x02]).is_ok());
/// ```
pub trait I2cBus {
    /// エラー型
    type Error;

    /// I2Cデバイスにデータを書き込む
    ///
    /// # Arguments
    ///
    /// * `addr` - I2Cデバイスアドレス（7ビット）
    /// * `bytes` - 書き込むデータ
    ///
    /// # Examples
    ///
    /// ```
    /// use hal_api::i2c::I2cBus;
    /// use hal_api::error::I2cError;
    ///
    /// struct TestI2c;
    ///
    /// impl I2cBus for TestI2c {
    ///     type Error = I2cError;
    ///     fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
    ///         Ok(())
    ///     }
    ///     fn read(&mut self, _: u8, _: &mut [u8]) -> Result<(), Self::Error> {
    ///         Ok(())
    ///     }
    ///     fn write_read(&mut self, _: u8, _: &[u8], _: &mut [u8]) -> Result<(), Self::Error> {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let mut i2c = TestI2c;
    /// assert!(i2c.write(0x48, &[0x01, 0x02]).is_ok());
    /// ```
    fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error>;

    /// I2Cデバイスからデータを読み取る
    ///
    /// # Arguments
    ///
    /// * `addr` - I2Cデバイスアドレス（7ビット）
    /// * `buffer` - データを格納するバッファ
    ///
    /// # Examples
    ///
    /// ```
    /// use hal_api::i2c::I2cBus;
    /// use hal_api::error::I2cError;
    ///
    /// struct TestI2c;
    ///
    /// impl I2cBus for TestI2c {
    ///     type Error = I2cError;
    ///     fn write(&mut self, _: u8, _: &[u8]) -> Result<(), Self::Error> {
    ///         Ok(())
    ///     }
    ///     fn read(&mut self, _: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
    ///         buffer.fill(0xFF);
    ///         Ok(())
    ///     }
    ///     fn write_read(&mut self, _: u8, _: &[u8], _: &mut [u8]) -> Result<(), Self::Error> {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let mut i2c = TestI2c;
    /// let mut buffer = [0u8; 4];
    /// assert!(i2c.read(0x48, &mut buffer).is_ok());
    /// assert_eq!(buffer, [0xFF, 0xFF, 0xFF, 0xFF]);
    /// ```
    fn read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), Self::Error>;

    /// I2Cデバイスにデータを書き込んだ後、データを読み取る
    ///
    /// # Arguments
    ///
    /// * `addr` - I2Cデバイスアドレス（7ビット）
    /// * `bytes` - 書き込むデータ（レジスタアドレスなど）
    /// * `buffer` - 読み取ったデータを格納するバッファ
    ///
    /// # Examples
    ///
    /// ```
    /// use hal_api::i2c::I2cBus;
    /// use hal_api::error::I2cError;
    ///
    /// struct TestI2c;
    ///
    /// impl I2cBus for TestI2c {
    ///     type Error = I2cError;
    ///     fn write(&mut self, _: u8, _: &[u8]) -> Result<(), Self::Error> {
    ///         Ok(())
    ///     }
    ///     fn read(&mut self, _: u8, _: &mut [u8]) -> Result<(), Self::Error> {
    ///         Ok(())
    ///     }
    ///     fn write_read(&mut self, _: u8, _: &[u8], buffer: &mut [u8]) -> Result<(), Self::Error> {
    ///         buffer.fill(0xAA);
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let mut i2c = TestI2c;
    /// let mut buffer = [0u8; 2];
    /// assert!(i2c.write_read(0x48, &[0x03], &mut buffer).is_ok());
    /// assert_eq!(buffer, [0xAA, 0xAA]);
    /// ```
    fn write_read(&mut self, addr: u8, bytes: &[u8], buffer: &mut [u8]) -> Result<(), Self::Error>;
}
