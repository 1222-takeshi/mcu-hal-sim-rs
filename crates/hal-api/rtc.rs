//! Real-Time Clock (RTC) sensor abstractions.

/// RTC の日時読み取り結果。
///
/// 年は 2000 年オフセット（`year_offset` = 0 → 2000年）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RtcDateTime {
    /// 2000年からのオフセット (0–99)
    pub year_offset: u8,
    /// 月 (1–12)
    pub month: u8,
    /// 日 (1–31)
    pub day: u8,
    /// 時 (0–23)
    pub hour: u8,
    /// 分 (0–59)
    pub minute: u8,
    /// 秒 (0–59)
    pub second: u8,
}

impl RtcDateTime {
    pub const fn new(
        year_offset: u8,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        second: u8,
    ) -> Self {
        Self {
            year_offset,
            month,
            day,
            hour,
            minute,
            second,
        }
    }

    /// 4 桁の年を返します（2000 + year_offset）。
    pub fn year(self) -> u16 {
        2000 + self.year_offset as u16
    }
}

/// DS3231 のような RTC センサの抽象。
///
/// # Examples
///
/// ```
/// use hal_api::rtc::{RtcDateTime, RtcSensor};
///
/// struct MockRtc;
///
/// impl RtcSensor for MockRtc {
///     type Error = ();
///     fn read_datetime(&mut self) -> Result<RtcDateTime, ()> {
///         Ok(RtcDateTime::new(25, 5, 4, 12, 0, 0))
///     }
///     fn set_datetime(&mut self, _dt: &RtcDateTime) -> Result<(), ()> {
///         Ok(())
///     }
/// }
///
/// let mut rtc = MockRtc;
/// let dt = rtc.read_datetime().unwrap();
/// assert_eq!(dt.year(), 2025);
/// assert_eq!(dt.month, 5);
/// ```
pub trait RtcSensor {
    type Error;

    fn read_datetime(&mut self) -> Result<RtcDateTime, Self::Error>;

    /// 日時を設定します。
    fn set_datetime(&mut self, dt: &RtcDateTime) -> Result<(), Self::Error>;
}
