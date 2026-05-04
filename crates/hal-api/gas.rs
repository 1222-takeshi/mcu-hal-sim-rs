//! Gas sensor (CO₂ / VOC) abstractions.

/// CO₂ / VOC センサの読み取り結果。
///
/// - `co2_ppm`: CO₂ 濃度 (parts per million)
/// - `voc_ppb`: VOC 濃度 (parts per billion)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GasReading {
    pub co2_ppm: u16,
    pub voc_ppb: u16,
}

impl GasReading {
    pub const fn new(co2_ppm: u16, voc_ppb: u16) -> Self {
        Self { co2_ppm, voc_ppb }
    }
}

/// SGP30 のような CO₂ / VOC センサの抽象。
///
/// # Examples
///
/// ```
/// use hal_api::gas::{GasReading, GasSensor};
///
/// struct MockGas;
///
/// impl GasSensor for MockGas {
///     type Error = ();
///     fn read_gas(&mut self) -> Result<GasReading, ()> {
///         Ok(GasReading::new(400, 0))
///     }
/// }
///
/// let mut sensor = MockGas;
/// let r = sensor.read_gas().unwrap();
/// assert_eq!(r.co2_ppm, 400);
/// ```
pub trait GasSensor {
    type Error;

    fn read_gas(&mut self) -> Result<GasReading, Self::Error>;
}
