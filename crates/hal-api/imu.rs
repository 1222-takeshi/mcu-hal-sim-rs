//! IMU sensor abstractions.

/// 6軸 IMU の読み取り結果。
///
/// 加速度は milli-g、角速度は milli-degrees-per-second で保持します。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImuReading {
    pub accel_mg: [i16; 3],
    pub gyro_mdps: [i32; 3],
    pub temperature_centi_celsius: Option<i16>,
}

impl ImuReading {
    pub const fn new(
        accel_mg: [i16; 3],
        gyro_mdps: [i32; 3],
        temperature_centi_celsius: Option<i16>,
    ) -> Self {
        Self {
            accel_mg,
            gyro_mdps,
            temperature_centi_celsius,
        }
    }
}

/// MPU6050 のような IMU センサの抽象。
pub trait ImuSensor {
    type Error;

    fn read_imu(&mut self) -> Result<ImuReading, Self::Error>;
}
