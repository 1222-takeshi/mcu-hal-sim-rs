//! Host-side LCD1602 mock device that decodes I2C backpack writes.

use crate::virtual_i2c::VirtualI2cDevice;
use hal_api::display::TextFrame16x2;
use hal_api::error::I2cError;
use reference_drivers::lcd1602::{BackpackMapping, Lcd1602Config};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Default)]
struct MockLcd1602State {
    config: Lcd1602Config,
    ddram: [[u8; 16]; 2],
    cursor_row: usize,
    cursor_col: usize,
    initialized: bool,
    backlight: bool,
    pending_nibble: Option<(u8, bool)>,
    last_enable_high: Option<u8>,
    write_count: usize,
}

#[derive(Clone, Debug)]
pub struct MockLcd1602Device {
    state: Rc<RefCell<MockLcd1602State>>,
}

impl MockLcd1602Device {
    pub fn new() -> Self {
        Self::new_with_config(Lcd1602Config::default())
    }

    pub fn new_with_config(config: Lcd1602Config) -> Self {
        Self {
            state: Rc::new(RefCell::new(MockLcd1602State {
                config,
                ddram: [[b' '; 16]; 2],
                cursor_row: 0,
                cursor_col: 0,
                initialized: false,
                backlight: config.backlight,
                pending_nibble: None,
                last_enable_high: None,
                write_count: 0,
            })),
        }
    }

    pub fn frame(&self) -> TextFrame16x2 {
        let state = self.state.borrow();
        let mut frame = TextFrame16x2::blank();
        for row in 0..2 {
            let line = core::str::from_utf8(&state.ddram[row]).unwrap_or("                ");
            frame.set_line(row, line);
        }
        frame
    }

    pub fn backlight_enabled(&self) -> bool {
        self.state.borrow().backlight
    }

    pub fn is_initialized(&self) -> bool {
        self.state.borrow().initialized
    }

    pub fn cursor_position(&self) -> (usize, usize) {
        let state = self.state.borrow();
        (state.cursor_row, state.cursor_col)
    }

    pub fn write_count(&self) -> usize {
        self.state.borrow().write_count
    }
}

impl Default for MockLcd1602Device {
    fn default() -> Self {
        Self::new()
    }
}

impl VirtualI2cDevice for MockLcd1602Device {
    fn write(&mut self, bytes: &[u8]) -> Result<(), I2cError> {
        let mut state = self.state.borrow_mut();
        for byte in bytes {
            state.write_count += 1;
            process_expander_byte(&mut state, *byte);
        }
        Ok(())
    }
}

fn process_expander_byte(state: &mut MockLcd1602State, byte: u8) {
    let mapping = state.config.mapping;
    state.backlight = byte & mapping.backlight != 0;

    let enable_high = byte & mapping.enable != 0;
    if enable_high {
        state.last_enable_high = Some(byte);
        return;
    }

    let Some(previous) = state.last_enable_high.take() else {
        return;
    };
    if previous & mapping.enable == 0 {
        return;
    }

    let register_select = byte & mapping.rs != 0;
    let nibble = decode_nibble(mapping, byte);
    if let Some((high_nibble, previous_rs)) = state.pending_nibble.take() {
        if previous_rs != register_select {
            state.pending_nibble = Some((nibble, register_select));
            return;
        }
        let value = (high_nibble << 4) | nibble;
        if register_select {
            write_data(state, value);
        } else {
            execute_command(state, value);
        }
    } else {
        state.pending_nibble = Some((nibble, register_select));
    }
}

fn decode_nibble(mapping: BackpackMapping, byte: u8) -> u8 {
    let mut nibble = 0u8;
    if byte & mapping.d4 != 0 {
        nibble |= 0x01;
    }
    if byte & mapping.d5 != 0 {
        nibble |= 0x02;
    }
    if byte & mapping.d6 != 0 {
        nibble |= 0x04;
    }
    if byte & mapping.d7 != 0 {
        nibble |= 0x08;
    }
    nibble
}

fn execute_command(state: &mut MockLcd1602State, command: u8) {
    match command {
        0x01 => {
            state.ddram = [[b' '; 16]; 2];
            state.cursor_row = 0;
            state.cursor_col = 0;
        }
        0x02 => {
            state.cursor_row = 0;
            state.cursor_col = 0;
        }
        0x0C => {
            state.initialized = true;
        }
        0x80..=0x8F => {
            state.cursor_row = 0;
            state.cursor_col = usize::from(command - 0x80).min(15);
        }
        0xC0..=0xCF => {
            state.cursor_row = 1;
            state.cursor_col = usize::from(command - 0xC0).min(15);
        }
        _ => {}
    }
}

fn write_data(state: &mut MockLcd1602State, value: u8) {
    if state.cursor_row < 2 && state.cursor_col < 16 {
        state.ddram[state.cursor_row][state.cursor_col] = value;
    }

    state.cursor_col += 1;
    if state.cursor_col >= 16 {
        state.cursor_col = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_hal::delay::DelayNs;
    use hal_api::display::TextDisplay16x2;
    use reference_drivers::lcd1602::{Lcd1602Display, LCD1602_ADDRESS_PRIMARY};

    #[derive(Default)]
    struct NoopDelay;

    impl DelayNs for NoopDelay {
        fn delay_ns(&mut self, _ns: u32) {}
        fn delay_us(&mut self, _us: u32) {}
        fn delay_ms(&mut self, _ms: u32) {}
    }

    #[test]
    fn lcd1602_mock_decodes_driver_writes_into_visible_frame() {
        let bus = crate::virtual_i2c::VirtualI2cBus::new();
        let device = MockLcd1602Device::new();
        bus.attach_device(LCD1602_ADDRESS_PRIMARY, device.clone());
        let mut display = Lcd1602Display::new(bus, NoopDelay);

        display
            .render(&TextFrame16x2::from_lines("Temp    24.8C", "Hum     43.1%"))
            .unwrap();

        assert!(device.is_initialized());
        assert_eq!(
            device.frame(),
            TextFrame16x2::from_lines("Temp    24.8C", "Hum     43.1%")
        );
        assert!(device.write_count() > 0);
    }

    #[test]
    fn lcd1602_mock_tracks_backlight_bit() {
        let mut device = MockLcd1602Device::new();
        let mapping = BackpackMapping::default();

        device.write(&[mapping.backlight]).unwrap();
        assert!(device.backlight_enabled());

        device.write(&[0x00]).unwrap();
        assert!(!device.backlight_enabled());
    }
}
