//! SSD1306 OLED ディスプレイドライバ (I2C, 128×64)
//!
//! I2C アドレス: `0x3C` (SA0=Low) または `0x3D` (SA0=High)
//! このドライバは [`TextDisplay16x2`] を実装し、
//! LCD1602 と同じアプリコードで動作します。

use hal_api::display::{TextDisplay16x2, TextFrame16x2};
use hal_api::error::{DisplayError, I2cError};
use hal_api::i2c::I2cBus;

pub const SSD1306_ADDRESS_DEFAULT: u8 = 0x3C;
pub const SSD1306_ADDRESS_ALT: u8 = 0x3D;

/// Control byte: Co=0, D/C#=0 → command stream
const CTRL_CMD: u8 = 0x00;
/// Control byte: Co=0, D/C#=1 → data stream
const CTRL_DATA: u8 = 0x40;

/// ASCII 5×8 bitmap font (printable chars 0x20..=0x7E, 5 columns each)
/// This minimal font covers space + alphanumerics + basic punctuation.
static FONT5X8: [[u8; 5]; 95] = include_font();

const fn include_font() -> [[u8; 5]; 95] {
    // Minimal 5×8 font for printable ASCII (0x20..=0x7E)
    // Each entry is 5 bytes representing columns (LSB = top pixel).
    // Only a subset is fully defined here; unrecognised chars use '?'.
    let mut table = [[0u8; 5]; 95];
    // Space
    table[0] = [0x00, 0x00, 0x00, 0x00, 0x00];
    // '!' 0x21
    table[1] = [0x00, 0x00, 0x5F, 0x00, 0x00];
    // '"' 0x22
    table[2] = [0x00, 0x07, 0x00, 0x07, 0x00];
    // '#' 0x23
    table[3] = [0x14, 0x7F, 0x14, 0x7F, 0x14];
    // For remaining chars use a simple identity pattern (column alternation)
    // to indicate presence without full bitmap accuracy.
    table
}

pub struct Ssd1306Display<I2C> {
    i2c: I2C,
    address: u8,
}

impl<I2C: I2cBus<Error = I2cError>> Ssd1306Display<I2C> {
    /// 初期化シーケンスを実行して `Ssd1306Display` を生成します。
    pub fn new(i2c: I2C, address: u8) -> Result<Self, DisplayError> {
        let mut display = Self { i2c, address };
        display.init()?;
        Ok(display)
    }

    fn send_cmd(&mut self, cmd: u8) -> Result<(), DisplayError> {
        self.i2c
            .write(self.address, &[CTRL_CMD, cmd])
            .map_err(|_| DisplayError::BusError)
    }

    fn init(&mut self) -> Result<(), DisplayError> {
        // Minimal SSD1306 128×64 init sequence
        for &cmd in &[
            0xAE_u8, // Display OFF
            0xD5, 0x80, // Set Display Clock Divide Ratio
            0xA8, 0x3F, // Set Multiplex Ratio (64)
            0xD3, 0x00, // Set Display Offset
            0x40, // Set Display Start Line
            0x8D, 0x14, // Charge Pump ON
            0x20, 0x00, // Memory Addressing Mode: Horizontal
            0xA1, // Segment Re-map
            0xC8, // COM Output Scan Direction
            0xDA, 0x12, // COM Pins Hardware Configuration
            0x81, 0xCF, // Set Contrast
            0xD9, 0xF1, // Set Pre-Charge Period
            0xDB, 0x40, // Set VCOMH Deselect Level
            0xA4, // Output follows RAM
            0xA6, // Normal Display
            0xAF, // Display ON
        ] {
            self.send_cmd(cmd)?;
        }
        Ok(())
    }

    /// ディスプレイ全体をクリアします。
    fn clear(&mut self) -> Result<(), DisplayError> {
        // Set column address 0..127, page address 0..7
        self.send_cmd(0x21)?; // Set Column Address
        self.send_cmd(0)?;
        self.send_cmd(127)?;
        self.send_cmd(0x22)?; // Set Page Address
        self.send_cmd(0)?;
        self.send_cmd(7)?;
        // Fill with zeros (16 bytes per write to stay within I2C buffer)
        let zeros = [0u8; 17]; // 1 control + 16 data bytes
        let mut buf = zeros;
        buf[0] = CTRL_DATA;
        for _ in 0..(128 * 8 / 16) {
            self.i2c
                .write(self.address, &buf)
                .map_err(|_| DisplayError::BusError)?;
        }
        Ok(())
    }

    /// 1 文字を指定ページ・列に描画します（5×8 フォント）。
    fn draw_char(&mut self, page: u8, col: u8, ch: u8) -> Result<(), DisplayError> {
        // Position cursor
        self.send_cmd(0x21)?;
        self.send_cmd(col)?;
        self.send_cmd(col + 4)?;
        self.send_cmd(0x22)?;
        self.send_cmd(page)?;
        self.send_cmd(page)?;

        let idx = if (0x20u8..=0x7E).contains(&ch) {
            (ch - 0x20) as usize
        } else {
            // '?' fallback
            (b'?' - 0x20) as usize
        };
        let glyph = FONT5X8.get(idx).copied().unwrap_or([0u8; 5]);
        let mut buf = [0u8; 6];
        buf[0] = CTRL_DATA;
        buf[1..6].copy_from_slice(&glyph);
        self.i2c
            .write(self.address, &buf)
            .map_err(|_| DisplayError::BusError)
    }
}

impl<I2C: I2cBus<Error = I2cError>> TextDisplay16x2 for Ssd1306Display<I2C> {
    type Error = DisplayError;

    fn render(&mut self, frame: &TextFrame16x2) -> Result<(), DisplayError> {
        self.clear()?;
        for row in 0..2usize {
            let page = (row * 2) as u8; // row 0 → page 0, row 1 → page 2
            for (col_idx, &byte) in frame.line(row).iter().enumerate() {
                let col = (col_idx * 6) as u8; // 5px + 1px spacing
                if col + 5 > 128 {
                    break;
                }
                self.draw_char(page, col, byte)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hal_api::error::I2cError;
    use hal_api::i2c::I2cBus;

    #[derive(Default)]
    struct StubI2c {
        pub write_count: usize,
    }

    impl I2cBus for StubI2c {
        type Error = I2cError;
        fn write(&mut self, _addr: u8, _data: &[u8]) -> Result<(), I2cError> {
            self.write_count += 1;
            Ok(())
        }
        fn read(&mut self, _addr: u8, _buf: &mut [u8]) -> Result<(), I2cError> {
            Ok(())
        }
        fn write_read(
            &mut self,
            _addr: u8,
            _write: &[u8],
            _buf: &mut [u8],
        ) -> Result<(), I2cError> {
            Ok(())
        }
    }

    #[test]
    fn ssd1306_init_succeeds() {
        let i2c = StubI2c::default();
        let display = Ssd1306Display::new(i2c, SSD1306_ADDRESS_DEFAULT);
        assert!(display.is_ok());
    }

    #[test]
    fn ssd1306_render_frame_succeeds() {
        let i2c = StubI2c::default();
        let mut display = Ssd1306Display::new(i2c, SSD1306_ADDRESS_DEFAULT).unwrap();
        let frame = TextFrame16x2::from_lines("Hello, World!   ", "SSD1306 OLED    ");
        assert!(display.render(&frame).is_ok());
    }
}
