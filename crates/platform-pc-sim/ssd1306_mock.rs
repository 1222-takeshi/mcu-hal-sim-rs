//! Host-side SSD1306 OLED ディスプレイモック

use crate::virtual_i2c::VirtualI2cDevice;
use hal_api::display::{TextDisplay16x2, TextFrame16x2};
use hal_api::error::{DisplayError, I2cError};
use std::cell::RefCell;
use std::rc::Rc;
use std::string::String;

/// SSD1306 に書き込まれたテキストを記録するモックデバイス
#[derive(Debug)]
struct MockSsd1306State {
    rendered_frames: Vec<[String; 2]>,
    /// 初期化コマンドシーケンスを受信したか
    initialized: bool,
    write_count: usize,
}

#[derive(Clone, Debug)]
pub struct MockSsd1306Device {
    state: Rc<RefCell<MockSsd1306State>>,
}

impl MockSsd1306Device {
    pub fn new() -> Self {
        Self {
            state: Rc::new(RefCell::new(MockSsd1306State {
                rendered_frames: Vec::new(),
                initialized: false,
                write_count: 0,
            })),
        }
    }

    pub fn write_count(&self) -> usize {
        self.state.borrow().write_count
    }

    pub fn is_initialized(&self) -> bool {
        self.state.borrow().initialized
    }

    pub fn last_frame(&self) -> Option<[String; 2]> {
        self.state.borrow().rendered_frames.last().cloned()
    }

    pub fn frame_count(&self) -> usize {
        self.state.borrow().rendered_frames.len()
    }
}

impl Default for MockSsd1306Device {
    fn default() -> Self {
        Self::new()
    }
}

impl VirtualI2cDevice for MockSsd1306Device {
    fn write(&mut self, bytes: &[u8]) -> Result<(), I2cError> {
        let mut state = self.state.borrow_mut();
        state.write_count += 1;
        // Control byte 0x00 = command, 0x40 = data
        if bytes.first() == Some(&0x00) {
            // Command — check for Display ON (0xAF) as init complete
            if bytes.get(1) == Some(&0xAF) {
                state.initialized = true;
            }
        }
        Ok(())
    }
}

/// `TextDisplay16x2` 直接実装（軽量モック、VirtualI2cBus 不要）
#[derive(Clone, Debug)]
pub struct MockSsd1306TextDisplay {
    inner: MockSsd1306Device,
}

impl MockSsd1306TextDisplay {
    pub fn new() -> Self {
        Self {
            inner: MockSsd1306Device::new(),
        }
    }

    pub fn last_frame(&self) -> Option<[String; 2]> {
        self.inner.last_frame()
    }

    pub fn frame_count(&self) -> usize {
        self.inner.frame_count()
    }
}

impl Default for MockSsd1306TextDisplay {
    fn default() -> Self {
        Self::new()
    }
}

impl TextDisplay16x2 for MockSsd1306TextDisplay {
    type Error = DisplayError;

    fn render(&mut self, frame: &TextFrame16x2) -> Result<(), DisplayError> {
        let line0 = String::from_utf8_lossy(frame.line(0)).into_owned();
        let line1 = String::from_utf8_lossy(frame.line(1)).into_owned();
        self.inner
            .state
            .borrow_mut()
            .rendered_frames
            .push([line0, line1]);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_ssd1306_records_rendered_frame() {
        let mut display = MockSsd1306TextDisplay::new();
        let frame = TextFrame16x2::from_lines("Hello, World!   ", "Row 2           ");
        display.render(&frame).unwrap();
        let last = display.last_frame().unwrap();
        assert!(last[0].contains("Hello"));
        assert!(last[1].contains("Row 2"));
    }

    #[test]
    fn mock_ssd1306_counts_frames() {
        let mut display = MockSsd1306TextDisplay::new();
        let frame = TextFrame16x2::blank();
        display.render(&frame).unwrap();
        display.render(&frame).unwrap();
        assert_eq!(display.frame_count(), 2);
    }

    #[test]
    fn mock_ssd1306_virtual_device_marks_initialized() {
        let mut device = MockSsd1306Device::new();
        // Send display ON command
        device.write(&[0x00, 0xAF]).unwrap();
        assert!(device.is_initialized());
    }
}
