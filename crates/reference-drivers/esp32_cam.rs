//! ESP32-CAM カメラドライバスタブ
//!
//! `CameraCapture` トレイトの ESP32-CAM 向け実装です。
//! PC シミュレータでは [`platform_pc_sim::camera_mock`] を使用します。
//! ESP32 実機向けには `platform-esp32` クレートで追加の初期化が必要です。

use hal_api::camera::{CameraCapture, FrameMetadata, PixelFormat};
use hal_api::error::SensorError;

/// ESP32-CAM デフォルト解像度: QVGA (320×240)
pub const ESP32CAM_WIDTH_DEFAULT: u32 = 320;
pub const ESP32CAM_HEIGHT_DEFAULT: u32 = 240;
/// JPEG 圧縮時のおよそのフレームサイズ（バイト）
pub const ESP32CAM_FRAME_SIZE_BYTES: u32 = 10_000;

/// ESP32-CAM ドライバ。
///
/// ハードウェア初期化・DMA バッファ管理は `platform-esp32` に委ねます。
/// このドライバはフレームメタデータの管理とシーケンス番号の更新を担当します。
pub struct Esp32CamSensor {
    width: u32,
    height: u32,
    format: PixelFormat,
    sequence: u32,
}

impl Esp32CamSensor {
    pub fn new(width: u32, height: u32, format: PixelFormat) -> Self {
        Self {
            width,
            height,
            format,
            sequence: 0,
        }
    }

    pub fn default_qvga() -> Self {
        Self::new(
            ESP32CAM_WIDTH_DEFAULT,
            ESP32CAM_HEIGHT_DEFAULT,
            PixelFormat::Jpeg,
        )
    }
}

impl CameraCapture for Esp32CamSensor {
    type Error = SensorError;

    fn capture_frame(&mut self) -> Result<FrameMetadata, SensorError> {
        self.sequence = self.sequence.wrapping_add(1);
        Ok(FrameMetadata::new(
            self.width,
            self.height,
            self.format,
            self.sequence,
            ESP32CAM_FRAME_SIZE_BYTES,
        ))
    }

    fn resolution(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn pixel_format(&self) -> PixelFormat {
        self.format
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn esp32cam_captures_sequential_frames() {
        let mut cam = Esp32CamSensor::default_qvga();
        let f1 = cam.capture_frame().unwrap();
        let f2 = cam.capture_frame().unwrap();
        assert_eq!(f1.sequence, 1);
        assert_eq!(f2.sequence, 2);
        assert_eq!(f1.width, 320);
        assert_eq!(f1.height, 240);
        assert_eq!(f1.format, PixelFormat::Jpeg);
    }

    #[test]
    fn esp32cam_resolution_matches_constructor() {
        let cam = Esp32CamSensor::new(640, 480, PixelFormat::Rgb565);
        assert_eq!(cam.resolution(), (640, 480));
        assert_eq!(cam.pixel_format(), PixelFormat::Rgb565);
    }
}
