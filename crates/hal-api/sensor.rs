//! センサ抽象

/// 温湿度センサの読み取り結果
///
/// 温度は摂氏の 1/100、湿度は %RH の 1/100 で保持します。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvReading {
    pub temperature_centi_celsius: i32,
    pub humidity_centi_percent: u32,
    pub pressure_pascal: Option<u32>,
}

impl EnvReading {
    pub const fn new(
        temperature_centi_celsius: i32,
        humidity_centi_percent: u32,
        pressure_pascal: Option<u32>,
    ) -> Self {
        Self {
            temperature_centi_celsius,
            humidity_centi_percent,
            pressure_pascal,
        }
    }
}

/// 温湿度センサの抽象
pub trait EnvSensor {
    type Error;

    fn read(&mut self) -> Result<EnvReading, Self::Error>;
}
