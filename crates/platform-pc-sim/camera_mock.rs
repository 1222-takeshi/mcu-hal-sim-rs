//! Host-side ESP32-CAM カメラモック

use hal_api::camera::{CameraCapture, FrameMetadata, PixelFormat};
use hal_api::error::SensorError;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug)]
struct MockCameraState {
    width: u32,
    height: u32,
    format: PixelFormat,
    sequence: u32,
    capture_count: usize,
    frame_size_bytes: u32,
}

#[derive(Clone, Debug)]
pub struct MockCamera {
    state: Rc<RefCell<MockCameraState>>,
}

impl MockCamera {
    pub fn new(width: u32, height: u32, format: PixelFormat) -> Self {
        Self {
            state: Rc::new(RefCell::new(MockCameraState {
                width,
                height,
                format,
                sequence: 0,
                capture_count: 0,
                frame_size_bytes: width * height / 8, // approximate 1bpp; real JPEG varies
            })),
        }
    }

    /// QVGA 320×240 JPEG モックを生成します。
    pub fn qvga_jpeg() -> Self {
        Self::new(320, 240, PixelFormat::Jpeg)
    }

    /// VGA 640×480 RGB565 モックを生成します。
    pub fn vga_rgb565() -> Self {
        Self::new(640, 480, PixelFormat::Rgb565)
    }

    pub fn capture_count(&self) -> usize {
        self.state.borrow().capture_count
    }

    pub fn sequence(&self) -> u32 {
        self.state.borrow().sequence
    }
}

impl Default for MockCamera {
    fn default() -> Self {
        Self::qvga_jpeg()
    }
}

impl CameraCapture for MockCamera {
    type Error = SensorError;

    fn capture_frame(&mut self) -> Result<FrameMetadata, SensorError> {
        let mut state = self.state.borrow_mut();
        state.sequence = state.sequence.wrapping_add(1);
        state.capture_count += 1;
        Ok(FrameMetadata::new(
            state.width,
            state.height,
            state.format,
            state.sequence,
            state.frame_size_bytes,
        ))
    }

    fn resolution(&self) -> (u32, u32) {
        let state = self.state.borrow();
        (state.width, state.height)
    }

    fn pixel_format(&self) -> PixelFormat {
        self.state.borrow().format
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_camera_increments_sequence() {
        let mut cam = MockCamera::qvga_jpeg();
        let f1 = cam.capture_frame().unwrap();
        let f2 = cam.capture_frame().unwrap();
        assert_eq!(f1.sequence, 1);
        assert_eq!(f2.sequence, 2);
    }

    #[test]
    fn mock_camera_returns_correct_resolution() {
        let cam = MockCamera::vga_rgb565();
        assert_eq!(cam.resolution(), (640, 480));
        assert_eq!(cam.pixel_format(), PixelFormat::Rgb565);
    }

    #[test]
    fn mock_camera_counts_captures() {
        let mut cam = MockCamera::default();
        cam.capture_frame().unwrap();
        cam.capture_frame().unwrap();
        cam.capture_frame().unwrap();
        assert_eq!(cam.capture_count(), 3);
    }
}
