//! LCD1602 (HD44780 + I2C backpack) ドライバ

use embedded_hal::delay::DelayNs;
use hal_api::display::{TextDisplay16x2, TextFrame16x2};
use hal_api::error::{DisplayError, I2cError};
use hal_api::i2c::I2cBus;

pub const LCD1602_ADDRESS_PRIMARY: u8 = 0x27;
pub const LCD1602_ADDRESS_SECONDARY: u8 = 0x3F;

/// よくある PCF8574 backpack のビット割り当て。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackpackMapping {
    pub rs: u8,
    pub rw: u8,
    pub enable: u8,
    pub backlight: u8,
    pub d4: u8,
    pub d5: u8,
    pub d6: u8,
    pub d7: u8,
}

impl BackpackMapping {
    fn encode_nibble(self, nibble: u8, register_select: bool, backlight: bool) -> u8 {
        let mut output = 0u8;
        if register_select {
            output |= self.rs;
        }
        if backlight {
            output |= self.backlight;
        }
        if nibble & 0x01 != 0 {
            output |= self.d4;
        }
        if nibble & 0x02 != 0 {
            output |= self.d5;
        }
        if nibble & 0x04 != 0 {
            output |= self.d6;
        }
        if nibble & 0x08 != 0 {
            output |= self.d7;
        }
        output
    }
}

impl Default for BackpackMapping {
    fn default() -> Self {
        Self {
            rs: 0x01,
            rw: 0x02,
            enable: 0x04,
            backlight: 0x08,
            d4: 0x10,
            d5: 0x20,
            d6: 0x40,
            d7: 0x80,
        }
    }
}

/// LCD1602 表示ドライバ。
pub struct Lcd1602Display<B, D> {
    bus: B,
    delay: D,
    address: u8,
    mapping: BackpackMapping,
    backlight: bool,
    initialized: bool,
}

impl<B, D> Lcd1602Display<B, D> {
    pub fn new(bus: B, delay: D) -> Self {
        Self::new_with_address(bus, delay, LCD1602_ADDRESS_PRIMARY)
    }

    pub fn new_with_address(bus: B, delay: D, address: u8) -> Self {
        Self {
            bus,
            delay,
            address,
            mapping: BackpackMapping::default(),
            backlight: true,
            initialized: false,
        }
    }

    pub fn new_with_mapping(bus: B, delay: D, address: u8, mapping: BackpackMapping) -> Self {
        Self {
            bus,
            delay,
            address,
            mapping,
            backlight: true,
            initialized: false,
        }
    }

    pub fn set_backlight(&mut self, enabled: bool) {
        self.backlight = enabled;
    }

    pub fn address(&self) -> u8 {
        self.address
    }

    #[cfg(test)]
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}

impl<B, D> Lcd1602Display<B, D>
where
    B: I2cBus<Error = I2cError>,
    D: DelayNs,
{
    fn initialize(&mut self) -> Result<(), DisplayError> {
        if self.initialized {
            return Ok(());
        }

        self.delay.delay_ms(50);
        self.write_init_nibble(0x03)?;
        self.delay.delay_ms(5);
        self.write_init_nibble(0x03)?;
        self.delay.delay_us(150);
        self.write_init_nibble(0x03)?;
        self.delay.delay_us(150);
        self.write_init_nibble(0x02)?;

        self.command(0x28)?; // 4-bit / 2-line / 5x8
        self.command(0x08)?; // display off
        self.command(0x01)?; // clear
        self.delay.delay_ms(2);
        self.command(0x06)?; // entry mode
        self.command(0x0C)?; // display on / cursor off

        self.initialized = true;
        Ok(())
    }

    fn write_init_nibble(&mut self, nibble: u8) -> Result<(), DisplayError> {
        let byte = self
            .mapping
            .encode_nibble(nibble & 0x0F, false, self.backlight);
        self.pulse_enable(byte)
    }

    fn command(&mut self, command: u8) -> Result<(), DisplayError> {
        self.write_byte(command, false)?;
        if matches!(command, 0x01 | 0x02) {
            self.delay.delay_ms(2);
        } else {
            self.delay.delay_us(50);
        }
        Ok(())
    }

    fn data(&mut self, byte: u8) -> Result<(), DisplayError> {
        self.write_byte(byte, true)?;
        self.delay.delay_us(50);
        Ok(())
    }

    fn write_byte(&mut self, byte: u8, register_select: bool) -> Result<(), DisplayError> {
        self.write_nibble(byte >> 4, register_select)?;
        self.write_nibble(byte & 0x0F, register_select)?;
        Ok(())
    }

    fn write_nibble(&mut self, nibble: u8, register_select: bool) -> Result<(), DisplayError> {
        let byte = self
            .mapping
            .encode_nibble(nibble & 0x0F, register_select, self.backlight);
        self.pulse_enable(byte)
    }

    fn pulse_enable(&mut self, byte: u8) -> Result<(), DisplayError> {
        self.bus
            .write(self.address, &[byte | self.mapping.enable])
            .map_err(map_display_error)?;
        self.delay.delay_us(1);
        self.bus
            .write(self.address, &[byte & !self.mapping.enable])
            .map_err(map_display_error)?;
        self.delay.delay_us(1);
        Ok(())
    }

    fn set_cursor(&mut self, row: usize) -> Result<(), DisplayError> {
        let base = match row {
            0 => 0x80,
            1 => 0xC0,
            _ => return Err(DisplayError::InvalidContent),
        };
        self.command(base)
    }
}

impl<B, D> TextDisplay16x2 for Lcd1602Display<B, D>
where
    B: I2cBus<Error = I2cError>,
    D: DelayNs,
{
    type Error = DisplayError;

    fn render(&mut self, frame: &TextFrame16x2) -> Result<(), Self::Error> {
        self.initialize()?;

        for row in 0..2 {
            self.set_cursor(row)?;
            for byte in frame.line(row) {
                self.data(*byte)?;
            }
        }

        Ok(())
    }
}

fn map_display_error(_error: I2cError) -> DisplayError {
    DisplayError::BusError
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::RefCell;
    use std::rc::Rc;
    use std::vec::Vec;

    #[derive(Clone, Default)]
    struct RecordingI2c {
        writes: Rc<RefCell<Vec<u8>>>,
        fail_after_writes: Rc<RefCell<Option<usize>>>,
    }

    impl I2cBus for RecordingI2c {
        type Error = I2cError;

        fn write(&mut self, _addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
            let mut fail_after_writes = self.fail_after_writes.borrow_mut();
            if let Some(remaining_writes) = fail_after_writes.as_mut() {
                if *remaining_writes == 0 {
                    return Err(I2cError::BusError);
                }
                *remaining_writes -= 1;
            }
            self.writes.borrow_mut().extend_from_slice(bytes);
            Ok(())
        }

        fn read(&mut self, _addr: u8, _buffer: &mut [u8]) -> Result<(), Self::Error> {
            Err(I2cError::BusError)
        }

        fn write_read(
            &mut self,
            _addr: u8,
            _bytes: &[u8],
            _buffer: &mut [u8],
        ) -> Result<(), Self::Error> {
            Err(I2cError::BusError)
        }
    }

    #[derive(Default)]
    struct DummyDelay {
        total_us: u32,
    }

    impl DelayNs for DummyDelay {
        fn delay_ns(&mut self, ns: u32) {
            self.total_us = self.total_us.saturating_add(ns / 1_000);
        }

        fn delay_us(&mut self, us: u32) {
            self.total_us = self.total_us.saturating_add(us);
        }

        fn delay_ms(&mut self, ms: u32) {
            self.total_us = self.total_us.saturating_add(ms.saturating_mul(1_000));
        }
    }

    #[test]
    fn default_mapping_matches_common_backpack_layout() {
        let mapping = BackpackMapping::default();

        assert_eq!(mapping.encode_nibble(0b1010, false, true), 0xA8);
        assert_eq!(mapping.encode_nibble(0b0001, true, true), 0x19);
    }

    #[test]
    fn lcd1602_display_initializes_once_and_renders() {
        let bus = RecordingI2c::default();
        let writes = bus.writes.clone();
        let delay = DummyDelay::default();
        let mut display = Lcd1602Display::new(bus, delay);
        let frame = TextFrame16x2::from_lines("Temp 24.8C", "Hum  43.2%");

        display.render(&frame).unwrap();
        let writes_after_first_render = writes.borrow().len();
        display.render(&frame).unwrap();
        let writes_after_second_render = writes.borrow().len() - writes_after_first_render;

        assert!(display.is_initialized());
        assert!(writes_after_first_render > writes_after_second_render);
        assert!(writes_after_second_render > 0);
    }

    #[test]
    fn lcd1602_display_can_disable_backlight() {
        let bus = RecordingI2c::default();
        let writes = bus.writes.clone();
        let delay = DummyDelay::default();
        let mut display = Lcd1602Display::new(bus, delay);
        display.set_backlight(false);

        display
            .render(&TextFrame16x2::from_lines("Line 1", "Line 2"))
            .unwrap();

        assert!(!writes
            .borrow()
            .contains(&BackpackMapping::default().backlight));
    }

    #[test]
    fn lcd1602_display_maps_i2c_failures_to_display_error() {
        let bus = RecordingI2c::default();
        *bus.fail_after_writes.borrow_mut() = Some(0);
        let delay = DummyDelay::default();
        let mut display = Lcd1602Display::new(bus, delay);

        assert_eq!(
            display.render(&TextFrame16x2::from_lines("Line 1", "Line 2")),
            Err(DisplayError::BusError)
        );
        assert!(!display.is_initialized());
    }
}
