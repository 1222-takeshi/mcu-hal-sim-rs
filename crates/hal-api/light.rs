//! 照度センサ抽象

/// 照度センサの読み取り結果。
///
/// `lux_x100` は lux の 100 倍（固定小数点）で保持します。
/// 例: 10000 は 100.00 lx を表します。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LightReading {
    pub lux_x100: u32,
}

impl LightReading {
    pub const fn new(lux_x100: u32) -> Self {
        Self { lux_x100 }
    }

    /// lux の整数部を返します。
    pub fn lux_integer(self) -> u32 {
        self.lux_x100 / 100
    }
}

/// 照度センサ（BH1750 等）の抽象。
///
/// # Examples
///
/// ```
/// use hal_api::light::{LightReading, LightSensor};
///
/// struct MockLight;
///
/// impl LightSensor for MockLight {
///     type Error = ();
///     fn read_lux(&mut self) -> Result<LightReading, ()> {
///         Ok(LightReading::new(5000)) // 50.00 lx
///     }
/// }
///
/// let mut sensor = MockLight;
/// let reading = sensor.read_lux().unwrap();
/// assert_eq!(reading.lux_integer(), 50);
/// ```
pub trait LightSensor {
    type Error;

    fn read_lux(&mut self) -> Result<LightReading, Self::Error>;
}
