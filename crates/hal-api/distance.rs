//! Distance sensor abstractions.

/// 距離センサの読み取り結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DistanceReading {
    pub distance_mm: u32,
}

impl DistanceReading {
    pub const fn new(distance_mm: u32) -> Self {
        Self { distance_mm }
    }
}

/// HC-SR04 のような距離センサの抽象。
pub trait DistanceSensor {
    type Error;

    fn read_distance(&mut self) -> Result<DistanceReading, Self::Error>;
}

/// HC-SR04 のような trigger / echo 型超音波センサ向けの低レベル抽象。
///
/// 実装側は trigger pulse の送出と echo high pulse 幅の測定を担当し、
/// driver 側は pulse 幅から距離へ変換します。
pub trait UltrasonicPulseDevice {
    type Error;

    fn trigger_and_measure_echo_us(&mut self) -> Result<u32, Self::Error>;
}
