//! IMU logging application — `ImuSensor` を使ったデータ収集とモーション検出。
//!
//! センサからの読み取りを一定間隔で行い、直近のサンプルをリングバッファに保持します。
//! 静止状態から逸脱したときに `motion_detected` フラグを立てます。
//!
//! # Examples
//!
//! ```
//! use core_app::imu_logger::{ImuLoggerApp, ImuLoggerConfig};
//! use hal_api::imu::{ImuReading, ImuSensor};
//!
//! struct MockImu;
//! impl ImuSensor for MockImu {
//!     type Error = ();
//!     fn read_imu(&mut self) -> Result<ImuReading, ()> {
//!         Ok(ImuReading::new([0, 0, 1000], [0, 0, 0], Some(2500)))
//!     }
//! }
//!
//! let mut app = ImuLoggerApp::new(MockImu);
//! for _ in 0..10 {
//!     app.tick().unwrap();
//! }
//! assert!(app.last_reading().is_some());
//! ```

use hal_api::imu::{ImuReading, ImuSensor};
use heapless::Vec;

#[cfg(test)]
extern crate std;

/// `ImuLoggerApp` の設定。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImuLoggerConfig {
    /// センサを読み取る tick 間隔。
    pub sample_period_ticks: u32,
    /// 静止状態 (1 g) からの逸脱検出しきい値 (mg)。
    pub motion_threshold_mg: u16,
}

impl Default for ImuLoggerConfig {
    fn default() -> Self {
        Self {
            sample_period_ticks: 10,
            motion_threshold_mg: 200,
        }
    }
}

/// `ImuLoggerApp` が返すエラー型。
#[derive(Debug, PartialEq, Eq)]
pub enum ImuLoggerError<E> {
    Sensor(E),
}

/// 直近の IMU サンプルを保持するリングバッファの容量。
pub const IMU_LOG_CAPACITY: usize = 10;

/// IMU ロギングアプリ。
///
/// `tick()` を毎ループ呼び出すことで、`sample_period_ticks` ごとにセンサを読み取り、
/// 直近 [`IMU_LOG_CAPACITY`] 件の読み取り結果を保持します。
pub struct ImuLoggerApp<IMU> {
    imu: IMU,
    tick_count: u32,
    config: ImuLoggerConfig,
    last_reading: Option<ImuReading>,
    log: Vec<ImuReading, IMU_LOG_CAPACITY>,
    motion_detected: bool,
}

impl<IMU> ImuLoggerApp<IMU>
where
    IMU: ImuSensor,
{
    pub fn new(imu: IMU) -> Self {
        Self::new_with_config(imu, ImuLoggerConfig::default())
    }

    pub fn new_with_config(imu: IMU, config: ImuLoggerConfig) -> Self {
        Self {
            imu,
            tick_count: 0,
            config,
            last_reading: None,
            log: Vec::new(),
            motion_detected: false,
        }
    }

    /// 1 tick 進める。`sample_period_ticks` の倍数 tick でセンサを読み取る。
    pub fn tick(&mut self) -> Result<(), ImuLoggerError<IMU::Error>> {
        self.tick_count += 1;
        let period = self.config.sample_period_ticks.max(1);
        if self.tick_count % period == 0 {
            let reading = self.imu.read_imu().map_err(ImuLoggerError::Sensor)?;
            self.motion_detected = detect_motion(&reading, self.config.motion_threshold_mg);
            if self.log.is_full() {
                self.log.remove(0);
            }
            self.log.push(reading).ok();
            self.last_reading = Some(reading);
        }
        Ok(())
    }

    /// 最新の読み取り結果を返す。まだ1回も読み取っていなければ `None`。
    pub fn last_reading(&self) -> Option<ImuReading> {
        self.last_reading
    }

    /// モーション検出フラグ。最後の読み取りで静止状態から逸脱していれば `true`。
    pub fn motion_detected(&self) -> bool {
        self.motion_detected
    }

    /// 直近のサンプルログ（最大 [`IMU_LOG_CAPACITY`] 件、古い順）。
    pub fn log(&self) -> &[ImuReading] {
        &self.log
    }

    pub fn tick_count(&self) -> u32 {
        self.tick_count
    }
}

/// 加速度ベクトルの大きさが 1 g から `threshold_mg` 以上離れているか判定する。
///
/// `|mag(accel) - 1000 mg| > threshold_mg` を整数演算で近似する。
/// 具体的には `|mag² - 1g²| > threshold * 2 * 1000` を使う。
fn detect_motion(reading: &ImuReading, threshold_mg: u16) -> bool {
    let [ax, ay, az] = reading.accel_mg;
    let mag_sq: i64 = ax as i64 * ax as i64 + ay as i64 * ay as i64 + az as i64 * az as i64;
    let one_g_sq: i64 = 1_000_000;
    let diff = (mag_sq - one_g_sq).unsigned_abs();
    diff > threshold_mg as u64 * 2_000
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockImu {
        readings: std::vec::Vec<ImuReading>,
        index: usize,
    }

    impl MockImu {
        fn new(readings: std::vec::Vec<ImuReading>) -> Self {
            Self { readings, index: 0 }
        }
    }

    impl ImuSensor for MockImu {
        type Error = ();
        fn read_imu(&mut self) -> Result<ImuReading, ()> {
            let r = self.readings[self.index % self.readings.len()];
            self.index += 1;
            Ok(r)
        }
    }

    fn at_rest() -> ImuReading {
        ImuReading::new([0, 0, 1000], [0, 0, 0], Some(2500))
    }

    fn in_motion() -> ImuReading {
        // accel magnitude >> 1g
        ImuReading::new([800, 800, 800], [500, 0, 0], None)
    }

    #[test]
    fn first_sample_available_after_one_period() {
        let mut app = ImuLoggerApp::new_with_config(
            MockImu::new(std::vec![at_rest()]),
            ImuLoggerConfig {
                sample_period_ticks: 5,
                ..Default::default()
            },
        );
        for _ in 0..4 {
            app.tick().unwrap();
            assert!(app.last_reading().is_none());
        }
        app.tick().unwrap();
        assert_eq!(app.last_reading(), Some(at_rest()));
        assert_eq!(app.tick_count(), 5);
    }

    #[test]
    fn log_fills_up_and_drops_oldest() {
        let readings = std::vec![at_rest(); IMU_LOG_CAPACITY + 3];
        let mut app = ImuLoggerApp::new_with_config(
            MockImu::new(readings),
            ImuLoggerConfig {
                sample_period_ticks: 1,
                ..Default::default()
            },
        );
        for _ in 0..(IMU_LOG_CAPACITY + 3) {
            app.tick().unwrap();
        }
        assert_eq!(app.log().len(), IMU_LOG_CAPACITY);
    }

    #[test]
    fn motion_detected_when_accel_deviates_from_1g() {
        let mut app = ImuLoggerApp::new_with_config(
            MockImu::new(std::vec![in_motion()]),
            ImuLoggerConfig {
                sample_period_ticks: 1,
                motion_threshold_mg: 200,
            },
        );
        app.tick().unwrap();
        assert!(app.motion_detected());
    }

    #[test]
    fn no_motion_when_at_rest() {
        let mut app = ImuLoggerApp::new_with_config(
            MockImu::new(std::vec![at_rest()]),
            ImuLoggerConfig {
                sample_period_ticks: 1,
                motion_threshold_mg: 200,
            },
        );
        app.tick().unwrap();
        assert!(!app.motion_detected());
    }

    #[test]
    fn motion_threshold_respected() {
        // 重力ベクトル方向に少しだけ傾けた状態 (50 mg ずれ)
        let small_tilt = ImuReading::new([50, 0, 998], [0, 0, 0], None);
        let mut app = ImuLoggerApp::new_with_config(
            MockImu::new(std::vec![small_tilt]),
            ImuLoggerConfig {
                sample_period_ticks: 1,
                motion_threshold_mg: 200,
            },
        );
        app.tick().unwrap();
        assert!(
            !app.motion_detected(),
            "small tilt should not trigger motion"
        );
    }

    #[test]
    fn detect_motion_fn_is_pure() {
        assert!(!detect_motion(&at_rest(), 200));
        assert!(detect_motion(&in_motion(), 200));
    }
}
