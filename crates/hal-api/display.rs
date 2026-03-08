//! 16x2 文字表示デバイス抽象

pub const DISPLAY_ROWS: usize = 2;
pub const DISPLAY_COLUMNS: usize = 16;
const BLANK_LINE: [u8; DISPLAY_COLUMNS] = [b' '; DISPLAY_COLUMNS];

/// 16x2 文字表示へ描画する固定フレーム
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextFrame16x2 {
    lines: [[u8; DISPLAY_COLUMNS]; DISPLAY_ROWS],
}

impl TextFrame16x2 {
    pub const fn blank() -> Self {
        Self {
            lines: [[b' '; DISPLAY_COLUMNS]; DISPLAY_ROWS],
        }
    }

    pub fn from_lines(line1: &str, line2: &str) -> Self {
        let mut frame = Self::blank();
        frame.set_line(0, line1);
        frame.set_line(1, line2);
        frame
    }

    pub fn line(&self, row: usize) -> &[u8; DISPLAY_COLUMNS] {
        self.line_checked(row).unwrap_or(&BLANK_LINE)
    }

    pub fn line_checked(&self, row: usize) -> Option<&[u8; DISPLAY_COLUMNS]> {
        self.lines.get(row)
    }

    pub fn set_line(&mut self, row: usize, text: &str) {
        let Some(target) = self.lines.get_mut(row) else {
            return;
        };
        target.fill(b' ');
        for (index, byte) in text
            .as_bytes()
            .iter()
            .copied()
            .take(DISPLAY_COLUMNS)
            .enumerate()
        {
            target[index] = byte;
        }
    }
}

/// 16x2 文字表示デバイスの抽象
pub trait TextDisplay16x2 {
    type Error;

    fn render(&mut self, frame: &TextFrame16x2) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_returns_blank_for_out_of_range_row() {
        let frame = TextFrame16x2::from_lines("Temp", "Hum");

        assert_eq!(frame.line(2), &BLANK_LINE);
        assert!(frame.line_checked(2).is_none());
    }

    #[test]
    fn set_line_ignores_out_of_range_row() {
        let mut frame = TextFrame16x2::from_lines("Temp", "Hum");

        frame.set_line(3, "Overflow");

        assert_eq!(frame.line(0)[..4], *b"Temp");
        assert_eq!(frame.line(1)[..3], *b"Hum");
    }
}
