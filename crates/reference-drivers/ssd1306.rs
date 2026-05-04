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
    // Covers printable ASCII 0x20..=0x7E. All 95 entries are defined.
    let mut table = [[0u8; 5]; 95];
    // ' ' (space)
    table[0] = [0x00, 0x00, 0x00, 0x00, 0x00];
    // '!' 0x21
    table[1] = [0x00, 0x00, 0x5F, 0x00, 0x00];
    // '"' 0x22
    table[2] = [0x00, 0x07, 0x00, 0x07, 0x00];
    // '#' 0x23
    table[3] = [0x14, 0x7F, 0x14, 0x7F, 0x14];
    // '$' 0x24
    table[4] = [0x24, 0x2A, 0x7F, 0x2A, 0x12];
    // '%' 0x25
    table[5] = [0x23, 0x13, 0x08, 0x64, 0x62];
    // '&' 0x26
    table[6] = [0x36, 0x49, 0x55, 0x22, 0x50];
    // '\'' 0x27
    table[7] = [0x00, 0x05, 0x03, 0x00, 0x00];
    // '(' 0x28
    table[8] = [0x00, 0x1C, 0x22, 0x41, 0x00];
    // ')' 0x29
    table[9] = [0x00, 0x41, 0x22, 0x1C, 0x00];
    // '*' 0x2A
    table[10] = [0x14, 0x08, 0x3E, 0x08, 0x14];
    // '+' 0x2B
    table[11] = [0x08, 0x08, 0x3E, 0x08, 0x08];
    // ',' 0x2C
    table[12] = [0x00, 0x50, 0x30, 0x00, 0x00];
    // '-' 0x2D
    table[13] = [0x08, 0x08, 0x08, 0x08, 0x08];
    // '.' 0x2E
    table[14] = [0x00, 0x60, 0x60, 0x00, 0x00];
    // '/' 0x2F
    table[15] = [0x20, 0x10, 0x08, 0x04, 0x02];
    // '0'-'9' (index 16-25)
    table[16] = [0x3E, 0x51, 0x49, 0x45, 0x3E];
    table[17] = [0x00, 0x42, 0x7F, 0x40, 0x00];
    table[18] = [0x72, 0x49, 0x49, 0x49, 0x46];
    table[19] = [0x21, 0x41, 0x49, 0x4D, 0x33];
    table[20] = [0x18, 0x14, 0x12, 0x7F, 0x10];
    table[21] = [0x27, 0x45, 0x45, 0x45, 0x39];
    table[22] = [0x3C, 0x4A, 0x49, 0x49, 0x31];
    table[23] = [0x41, 0x21, 0x11, 0x09, 0x07];
    table[24] = [0x36, 0x49, 0x49, 0x49, 0x36];
    table[25] = [0x46, 0x49, 0x49, 0x29, 0x1E];
    // ':' 0x3A
    table[26] = [0x00, 0x36, 0x36, 0x00, 0x00];
    // ';' 0x3B
    table[27] = [0x00, 0x56, 0x36, 0x00, 0x00];
    // '<' 0x3C
    table[28] = [0x08, 0x14, 0x22, 0x41, 0x00];
    // '=' 0x3D
    table[29] = [0x14, 0x14, 0x14, 0x14, 0x14];
    // '>' 0x3E
    table[30] = [0x00, 0x41, 0x22, 0x14, 0x08];
    // '?' fallback (index 31) — must be non-zero for reliable fallback
    table[31] = [0x02, 0x01, 0x51, 0x09, 0x06];
    // '@' 0x40
    table[32] = [0x32, 0x49, 0x79, 0x41, 0x3E];
    // 'A'-'Z' (index 33-58)
    table[33] = [0x7E, 0x11, 0x11, 0x11, 0x7E];
    table[34] = [0x7F, 0x49, 0x49, 0x49, 0x36];
    table[35] = [0x3E, 0x41, 0x41, 0x41, 0x22];
    table[36] = [0x7F, 0x41, 0x41, 0x22, 0x1C];
    table[37] = [0x7F, 0x49, 0x49, 0x49, 0x41];
    table[38] = [0x7F, 0x09, 0x09, 0x09, 0x01];
    table[39] = [0x3E, 0x41, 0x49, 0x49, 0x7A];
    table[40] = [0x7F, 0x08, 0x08, 0x08, 0x7F];
    table[41] = [0x00, 0x41, 0x7F, 0x41, 0x00];
    table[42] = [0x20, 0x40, 0x41, 0x3F, 0x01];
    table[43] = [0x7F, 0x08, 0x14, 0x22, 0x41];
    table[44] = [0x7F, 0x40, 0x40, 0x40, 0x40];
    table[45] = [0x7F, 0x02, 0x0C, 0x02, 0x7F];
    table[46] = [0x7F, 0x04, 0x08, 0x10, 0x7F];
    table[47] = [0x3E, 0x41, 0x41, 0x41, 0x3E];
    table[48] = [0x7F, 0x09, 0x09, 0x09, 0x06];
    table[49] = [0x3E, 0x41, 0x51, 0x21, 0x5E];
    table[50] = [0x7F, 0x09, 0x19, 0x29, 0x46];
    table[51] = [0x46, 0x49, 0x49, 0x49, 0x31];
    table[52] = [0x01, 0x01, 0x7F, 0x01, 0x01];
    table[53] = [0x3F, 0x40, 0x40, 0x40, 0x3F];
    table[54] = [0x1F, 0x20, 0x40, 0x20, 0x1F];
    table[55] = [0x3F, 0x40, 0x38, 0x40, 0x3F];
    table[56] = [0x63, 0x14, 0x08, 0x14, 0x63];
    table[57] = [0x07, 0x08, 0x70, 0x08, 0x07];
    table[58] = [0x61, 0x51, 0x49, 0x45, 0x43];
    // '[' 0x5B
    table[59] = [0x00, 0x7F, 0x41, 0x41, 0x00];
    // '\\' 0x5C
    table[60] = [0x02, 0x04, 0x08, 0x10, 0x20];
    // ']' 0x5D
    table[61] = [0x00, 0x41, 0x41, 0x7F, 0x00];
    // '^' 0x5E
    table[62] = [0x04, 0x02, 0x01, 0x02, 0x04];
    // '_' 0x5F
    table[63] = [0x40, 0x40, 0x40, 0x40, 0x40];
    // '`' 0x60
    table[64] = [0x00, 0x01, 0x02, 0x04, 0x00];
    // 'a'-'z' (index 65-90)
    table[65] = [0x20, 0x54, 0x54, 0x54, 0x78];
    table[66] = [0x7F, 0x48, 0x44, 0x44, 0x38];
    table[67] = [0x38, 0x44, 0x44, 0x44, 0x20];
    table[68] = [0x38, 0x44, 0x44, 0x48, 0x7F];
    table[69] = [0x38, 0x54, 0x54, 0x54, 0x18];
    table[70] = [0x08, 0x7E, 0x09, 0x01, 0x02];
    table[71] = [0x0C, 0x52, 0x52, 0x52, 0x3E];
    table[72] = [0x7F, 0x08, 0x04, 0x04, 0x78];
    table[73] = [0x00, 0x44, 0x7D, 0x40, 0x00];
    table[74] = [0x20, 0x40, 0x44, 0x3D, 0x00];
    table[75] = [0x7F, 0x10, 0x28, 0x44, 0x00];
    table[76] = [0x00, 0x41, 0x7F, 0x40, 0x00];
    table[77] = [0x7C, 0x04, 0x18, 0x04, 0x78];
    table[78] = [0x7C, 0x08, 0x04, 0x04, 0x78];
    table[79] = [0x38, 0x44, 0x44, 0x44, 0x38];
    table[80] = [0x7C, 0x14, 0x14, 0x14, 0x08];
    table[81] = [0x08, 0x14, 0x14, 0x18, 0x7C];
    table[82] = [0x7C, 0x08, 0x04, 0x04, 0x08];
    table[83] = [0x48, 0x54, 0x54, 0x54, 0x20];
    table[84] = [0x04, 0x3F, 0x44, 0x40, 0x20];
    table[85] = [0x3C, 0x40, 0x40, 0x20, 0x7C];
    table[86] = [0x1C, 0x20, 0x40, 0x20, 0x1C];
    table[87] = [0x3C, 0x40, 0x30, 0x40, 0x3C];
    table[88] = [0x44, 0x28, 0x10, 0x28, 0x44];
    table[89] = [0x0C, 0x50, 0x50, 0x50, 0x3C];
    table[90] = [0x44, 0x64, 0x54, 0x4C, 0x44];
    // '{' 0x7B
    table[91] = [0x00, 0x08, 0x36, 0x41, 0x00];
    // '|' 0x7C
    table[92] = [0x00, 0x00, 0x7F, 0x00, 0x00];
    // '}' 0x7D
    table[93] = [0x00, 0x41, 0x36, 0x08, 0x00];
    // '~' 0x7E
    table[94] = [0x10, 0x08, 0x08, 0x10, 0x08];
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

    #[test]
    fn ssd1306_font_digit_is_nonzero() {
        let table = include_font();
        // Digits '0'-'9' are at index 16-25
        for (i, glyph) in table[16..=25].iter().enumerate() {
            assert_ne!(
                *glyph,
                [0u8; 5],
                "Digit glyph at index {} should be non-zero",
                i + 16
            );
        }
    }

    #[test]
    fn ssd1306_font_question_mark_is_nonzero() {
        let table = include_font();
        // '?' is at 0x3F - 0x20 = 31
        assert_ne!(table[31], [0u8; 5], "'?' fallback glyph should be non-zero");
    }

    #[test]
    fn ssd1306_font_uppercase_a_is_nonzero() {
        let table = include_font();
        // 'A'-'Z' are at index 33-58
        for (i, glyph) in table[33..=58].iter().enumerate() {
            assert_ne!(
                *glyph,
                [0u8; 5],
                "Uppercase glyph at index {} should be non-zero",
                i + 33
            );
        }
    }
}
