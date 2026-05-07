//! カメラキャプチャ抽象

/// カメラのピクセルフォーマット。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Rgb565,
    Jpeg,
    Grayscale,
}

impl PixelFormat {
    /// フォーマット名の短い文字列表現を返します。
    ///
    /// # Examples
    ///
    /// ```
    /// use hal_api::camera::PixelFormat;
    /// assert_eq!(PixelFormat::Jpeg.as_str(), "JPEG");
    /// ```
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rgb565 => "RGB565",
            Self::Jpeg => "JPEG",
            Self::Grayscale => "GRAY",
        }
    }
}

/// キャプチャしたフレームのメタデータ。
///
/// 実際の画像データは含みません（メモリ効率のため）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameMetadata {
    pub width: u32,
    pub height: u32,
    pub format: PixelFormat,
    /// 単調増加するフレームシーケンス番号。
    pub sequence: u32,
    /// フレームのバイトサイズ（JPEG では可変）。
    pub size_bytes: u32,
}

impl FrameMetadata {
    pub const fn new(
        width: u32,
        height: u32,
        format: PixelFormat,
        sequence: u32,
        size_bytes: u32,
    ) -> Self {
        Self {
            width,
            height,
            format,
            sequence,
            size_bytes,
        }
    }
}

/// カメラキャプチャデバイス（ESP32-CAM 等）の抽象。
///
/// # Examples
///
/// ```
/// use hal_api::camera::{CameraCapture, FrameMetadata, PixelFormat};
///
/// struct MockCam { seq: u32 }
///
/// impl CameraCapture for MockCam {
///     type Error = ();
///     fn capture_frame(&mut self) -> Result<FrameMetadata, ()> {
///         self.seq += 1;
///         Ok(FrameMetadata::new(320, 240, PixelFormat::Jpeg, self.seq, 12000))
///     }
///     fn resolution(&self) -> (u32, u32) { (320, 240) }
///     fn pixel_format(&self) -> PixelFormat { PixelFormat::Jpeg }
/// }
///
/// let mut cam = MockCam { seq: 0 };
/// let frame = cam.capture_frame().unwrap();
/// assert_eq!(frame.width, 320);
/// assert_eq!(frame.sequence, 1);
/// ```
pub trait CameraCapture {
    type Error;

    /// フレームをキャプチャし、メタデータを返します。
    fn capture_frame(&mut self) -> Result<FrameMetadata, Self::Error>;

    /// 現在の解像度 `(width, height)` を返します。
    fn resolution(&self) -> (u32, u32);

    /// ピクセルフォーマットを返します。
    fn pixel_format(&self) -> PixelFormat;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_format_as_str_covers_all_variants() {
        assert_eq!(PixelFormat::Rgb565.as_str(), "RGB565");
        assert_eq!(PixelFormat::Jpeg.as_str(), "JPEG");
        assert_eq!(PixelFormat::Grayscale.as_str(), "GRAY");
    }

    #[test]
    fn frame_metadata_new_stores_fields() {
        let m = FrameMetadata::new(640, 480, PixelFormat::Rgb565, 7, 614400);
        assert_eq!(m.width, 640);
        assert_eq!(m.height, 480);
        assert_eq!(m.format, PixelFormat::Rgb565);
        assert_eq!(m.sequence, 7);
        assert_eq!(m.size_bytes, 614400);
    }

    #[test]
    fn pixel_format_equality() {
        assert_eq!(PixelFormat::Jpeg, PixelFormat::Jpeg);
        assert_ne!(PixelFormat::Jpeg, PixelFormat::Grayscale);
    }
}
