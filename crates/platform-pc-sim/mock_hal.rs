use hal_api::error::{GpioError, I2cError};
use hal_api::gpio::OutputPin;
use hal_api::i2c::I2cBus;

pub struct MockPin {
    pin_number: u8,
    state: bool,
}

impl MockPin {
    pub fn new(pin_number: u8) -> Self {
        Self {
            pin_number,
            state: false,
        }
    }

    /// テスト用: ピン番号を取得
    #[cfg(test)]
    pub fn pin_number(&self) -> u8 {
        self.pin_number
    }

    /// テスト用: 現在の状態を取得
    #[cfg(test)]
    pub fn state(&self) -> bool {
        self.state
    }
}

impl OutputPin for MockPin {
    type Error = GpioError;

    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.state = true;
        println!("[GPIO] Pin {} set HIGH", self.pin_number);
        Ok(())
    }

    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.state = false;
        println!("[GPIO] Pin {} set LOW", self.pin_number);
        Ok(())
    }
}

pub struct MockI2c;

impl MockI2c {
    pub fn new() -> Self {
        Self
    }
}

impl I2cBus for MockI2c {
    type Error = I2cError;

    fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        println!("[I2C] Write to 0x{:02X}: {:?}", addr, bytes);
        Ok(())
    }

    fn read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        println!("[I2C] Read from 0x{:02X}: {} bytes", addr, buffer.len());
        buffer.fill(0xFF);
        Ok(())
    }

    fn write_read(&mut self, addr: u8, bytes: &[u8], buffer: &mut [u8]) -> Result<(), Self::Error> {
        self.write(addr, bytes)?;
        self.read(addr, buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== MockPin Tests =====

    #[test]
    fn test_mock_pin_new() {
        let pin = MockPin::new(13);
        assert_eq!(pin.pin_number(), 13);
        assert!(!pin.state());
    }

    #[test]
    fn test_mock_pin_new_different_pin_numbers() {
        let pin1 = MockPin::new(5);
        let pin2 = MockPin::new(42);
        assert_eq!(pin1.pin_number(), 5);
        assert_eq!(pin2.pin_number(), 42);
    }

    #[test]
    fn test_mock_pin_set_high() {
        let mut pin = MockPin::new(13);
        assert!(pin.set_high().is_ok());
        assert!(pin.state());
    }

    #[test]
    fn test_mock_pin_set_low() {
        let mut pin = MockPin::new(13);
        pin.state = true; // 初期状態をHIGHに設定
        assert!(pin.set_low().is_ok());
        assert!(!pin.state());
    }

    #[test]
    fn test_mock_pin_set_with_true() {
        let mut pin = MockPin::new(13);
        assert!(pin.set(true).is_ok());
        assert!(pin.state());
    }

    #[test]
    fn test_mock_pin_set_with_false() {
        let mut pin = MockPin::new(13);
        pin.state = true;
        assert!(pin.set(false).is_ok());
        assert!(!pin.state());
    }

    #[test]
    fn test_mock_pin_toggle_sequence() {
        let mut pin = MockPin::new(13);

        pin.set_high().unwrap();
        assert!(pin.state());

        pin.set_low().unwrap();
        assert!(!pin.state());

        pin.set_high().unwrap();
        assert!(pin.state());
    }

    #[test]
    fn test_mock_pin_multiple_set_high() {
        let mut pin = MockPin::new(13);
        pin.set_high().unwrap();
        pin.set_high().unwrap();
        pin.set_high().unwrap();
        assert!(pin.state());
    }

    #[test]
    fn test_mock_pin_implements_output_pin_trait() {
        fn accepts_output_pin<T: OutputPin>(pin: &mut T) -> bool {
            pin.set_high().is_ok()
        }

        let mut pin = MockPin::new(13);
        assert!(accepts_output_pin(&mut pin));
    }

    // ===== MockI2c Tests =====

    #[test]
    fn test_mock_i2c_new() {
        let _i2c = MockI2c::new();
        // MockI2cは状態を持たないので、作成できることを確認
    }

    #[test]
    fn test_mock_i2c_write() {
        let mut i2c = MockI2c::new();
        let data = [0x01, 0x02, 0x03];
        assert!(i2c.write(0x48, &data).is_ok());
    }

    #[test]
    fn test_mock_i2c_write_empty() {
        let mut i2c = MockI2c::new();
        assert!(i2c.write(0x48, &[]).is_ok());
    }

    #[test]
    fn test_mock_i2c_read() {
        let mut i2c = MockI2c::new();
        let mut buffer = [0u8; 4];
        assert!(i2c.read(0x48, &mut buffer).is_ok());
        assert_eq!(buffer, [0xFF, 0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn test_mock_i2c_read_fills_buffer_with_0xff() {
        let mut i2c = MockI2c::new();
        let mut buffer = [0x00, 0x11, 0x22, 0x33];
        i2c.read(0x48, &mut buffer).unwrap();
        assert_eq!(buffer, [0xFF, 0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn test_mock_i2c_read_different_sizes() {
        let mut i2c = MockI2c::new();

        let mut buffer1 = [0u8; 1];
        i2c.read(0x48, &mut buffer1).unwrap();
        assert_eq!(buffer1, [0xFF]);

        let mut buffer8 = [0u8; 8];
        i2c.read(0x48, &mut buffer8).unwrap();
        assert_eq!(buffer8, [0xFF; 8]);
    }

    #[test]
    fn test_mock_i2c_write_read() {
        let mut i2c = MockI2c::new();
        let write_data = [0x03];
        let mut read_buffer = [0u8; 2];

        assert!(i2c.write_read(0x48, &write_data, &mut read_buffer).is_ok());
        assert_eq!(read_buffer, [0xFF, 0xFF]);
    }

    #[test]
    fn test_mock_i2c_write_read_empty_write() {
        let mut i2c = MockI2c::new();
        let mut read_buffer = [0u8; 2];

        assert!(i2c.write_read(0x48, &[], &mut read_buffer).is_ok());
        assert_eq!(read_buffer, [0xFF, 0xFF]);
    }

    #[test]
    fn test_mock_i2c_different_addresses() {
        let mut i2c = MockI2c::new();
        let mut buffer = [0u8; 2];

        assert!(i2c.read(0x20, &mut buffer).is_ok());
        assert!(i2c.read(0x48, &mut buffer).is_ok());
        assert!(i2c.read(0x77, &mut buffer).is_ok());
    }

    #[test]
    fn test_mock_i2c_implements_i2c_bus_trait() {
        fn accepts_i2c_bus<T: I2cBus>(i2c: &mut T) -> bool {
            let mut buffer = [0u8; 1];
            i2c.read(0x48, &mut buffer).is_ok()
        }

        let mut i2c = MockI2c::new();
        assert!(accepts_i2c_bus(&mut i2c));
    }

    #[test]
    fn test_mock_i2c_multiple_operations() {
        let mut i2c = MockI2c::new();

        i2c.write(0x48, &[0x01]).unwrap();
        let mut buffer = [0u8; 2];
        i2c.read(0x48, &mut buffer).unwrap();
        i2c.write_read(0x48, &[0x02], &mut buffer).unwrap();

        assert_eq!(buffer, [0xFF, 0xFF]);
    }
}
