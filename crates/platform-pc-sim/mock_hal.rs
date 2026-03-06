//! # モックHAL実装
//!
//! PCシミュレータ用のHAL trait実装。
//!
//! このモジュールは、実際のハードウェアを持たないPC環境で
//! アプリケーションをテスト・実行するためのモック実装を提供します。
//!
//! ## 提供する型
//!
//! - [`MockPin`]: GPIO出力ピンのモック実装
//! - [`MockI2c`]: I2Cバスのモック実装

use hal_api::error::{GpioError, I2cError};
use hal_api::gpio::OutputPin;
use hal_api::i2c::I2cBus;
use std::cell::RefCell;
use std::rc::Rc;
use std::vec::Vec;

#[derive(Debug, Default)]
struct MockPinState {
    level: bool,
    history: Vec<bool>,
}

/// GPIO出力ピンのモック実装
///
/// ピンの状態変更をコンソールに出力します。
/// クローンしたインスタンス間では内部状態が共有されるため、
/// テストやシミュレータから実行結果を観測できます。
///
/// # Examples
///
/// ```
/// use platform_pc_sim::mock_hal::MockPin;
/// use hal_api::gpio::OutputPin;
///
/// let mut pin = MockPin::new(13);
/// pin.set_high().unwrap();
/// pin.set_low().unwrap();
/// assert_eq!(pin.history(), vec![true, false]);
/// ```
#[derive(Clone, Debug)]
pub struct MockPin {
    pin_number: u8,
    state: Rc<RefCell<MockPinState>>,
}

impl MockPin {
    /// 新しいモックピンを作成
    ///
    /// # Arguments
    ///
    /// - `pin_number`: ピン番号（ログ出力用）
    ///
    /// # Examples
    ///
    /// ```
    /// use platform_pc_sim::mock_hal::MockPin;
    ///
    /// let pin = MockPin::new(13);
    /// assert_eq!(pin.pin_number(), 13);
    /// ```
    pub fn new(pin_number: u8) -> Self {
        Self {
            pin_number,
            state: Rc::new(RefCell::new(MockPinState::default())),
        }
    }

    /// ピン番号を取得
    pub fn pin_number(&self) -> u8 {
        self.pin_number
    }

    /// 現在の出力レベルを取得
    pub fn level(&self) -> bool {
        self.state.borrow().level
    }

    /// 出力履歴を取得
    pub fn history(&self) -> Vec<bool> {
        self.state.borrow().history.clone()
    }
}

impl OutputPin for MockPin {
    type Error = GpioError;

    fn set_high(&mut self) -> Result<(), Self::Error> {
        let mut state = self.state.borrow_mut();
        state.level = true;
        state.history.push(true);
        println!("[GPIO] Pin {} set HIGH", self.pin_number);
        Ok(())
    }

    fn set_low(&mut self) -> Result<(), Self::Error> {
        let mut state = self.state.borrow_mut();
        state.level = false;
        state.history.push(false);
        println!("[GPIO] Pin {} set LOW", self.pin_number);
        Ok(())
    }
}

#[derive(Debug, Default)]
struct MockI2cState {
    read_count: usize,
    last_read_addr: Option<u8>,
    last_read_len: Option<usize>,
    writes: Vec<(u8, Vec<u8>)>,
}

/// I2Cバスのモック実装
///
/// I2C通信をコンソールに出力します。
/// 読み取り操作では、バッファを0xFFで埋めます。
/// クローンしたインスタンス間で観測状態を共有します。
///
/// # Examples
///
/// ```
/// use platform_pc_sim::mock_hal::MockI2c;
/// use hal_api::i2c::I2cBus;
///
/// let mut i2c = MockI2c::new();
/// let mut buffer = [0u8; 4];
/// i2c.read(0x48, &mut buffer).unwrap();
/// assert_eq!(buffer, [0xFF, 0xFF, 0xFF, 0xFF]);
/// assert_eq!(i2c.read_count(), 1);
/// ```
#[derive(Clone, Debug)]
pub struct MockI2c {
    state: Rc<RefCell<MockI2cState>>,
}

impl MockI2c {
    /// 新しいモックI2Cバスを作成
    ///
    /// # Examples
    ///
    /// ```
    /// use platform_pc_sim::mock_hal::MockI2c;
    ///
    /// let i2c = MockI2c::new();
    /// assert_eq!(i2c.read_count(), 0);
    /// ```
    pub fn new() -> Self {
        Self {
            state: Rc::new(RefCell::new(MockI2cState::default())),
        }
    }

    /// 読み取り回数を取得
    pub fn read_count(&self) -> usize {
        self.state.borrow().read_count
    }

    /// 最後に読み取ったアドレスを取得
    pub fn last_read_addr(&self) -> Option<u8> {
        self.state.borrow().last_read_addr
    }

    /// 最後に読み取ったバイト数を取得
    pub fn last_read_len(&self) -> Option<usize> {
        self.state.borrow().last_read_len
    }

    /// 書き込み履歴を取得
    pub fn writes(&self) -> Vec<(u8, Vec<u8>)> {
        self.state.borrow().writes.clone()
    }
}

impl I2cBus for MockI2c {
    type Error = I2cError;

    fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        self.state.borrow_mut().writes.push((addr, bytes.to_vec()));
        println!("[I2C] Write to 0x{:02X}: {:?}", addr, bytes);
        Ok(())
    }

    fn read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        let mut state = self.state.borrow_mut();
        state.read_count += 1;
        state.last_read_addr = Some(addr);
        state.last_read_len = Some(buffer.len());
        println!("[I2C] Read from 0x{:02X}: {} bytes", addr, buffer.len());
        buffer.fill(0xFF);
        Ok(())
    }

    fn write_read(
        &mut self,
        addr: u8,
        bytes: &[u8],
        buffer: &mut [u8],
    ) -> Result<(), Self::Error> {
        self.write(addr, bytes)?;
        self.read(addr, buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_pin_new() {
        let pin = MockPin::new(13);
        assert_eq!(pin.pin_number(), 13);
        assert!(!pin.level());
        assert!(pin.history().is_empty());
    }

    #[test]
    fn test_mock_pin_state_changes() {
        let mut pin = MockPin::new(13);

        pin.set_high().unwrap();
        assert!(pin.level());

        pin.set_low().unwrap();
        assert!(!pin.level());

        assert_eq!(pin.history(), vec![true, false]);
    }

    #[test]
    fn test_mock_pin_set_helper() {
        let mut pin = MockPin::new(13);

        pin.set(true).unwrap();
        pin.set(false).unwrap();

        assert_eq!(pin.history(), vec![true, false]);
    }

    #[test]
    fn test_mock_pin_clone_shares_state() {
        let mut pin = MockPin::new(13);
        let observer = pin.clone();

        pin.set_high().unwrap();

        assert!(observer.level());
        assert_eq!(observer.history(), vec![true]);
    }

    #[test]
    fn test_mock_pin_implements_output_pin_trait() {
        fn accepts_output_pin<T: OutputPin>(pin: &mut T) -> bool {
            pin.set_high().is_ok()
        }

        let mut pin = MockPin::new(13);
        assert!(accepts_output_pin(&mut pin));
    }

    #[test]
    fn test_mock_i2c_new() {
        let i2c = MockI2c::new();
        assert_eq!(i2c.read_count(), 0);
        assert_eq!(i2c.last_read_addr(), None);
        assert_eq!(i2c.last_read_len(), None);
        assert!(i2c.writes().is_empty());
    }

    #[test]
    fn test_mock_i2c_write_tracks_history() {
        let mut i2c = MockI2c::new();
        i2c.write(0x48, &[0x01, 0x02]).unwrap();

        assert_eq!(i2c.writes(), vec![(0x48, vec![0x01, 0x02])]);
    }

    #[test]
    fn test_mock_i2c_read_tracks_metadata() {
        let mut i2c = MockI2c::new();
        let mut buffer = [0u8; 4];

        i2c.read(0x48, &mut buffer).unwrap();

        assert_eq!(buffer, [0xFF; 4]);
        assert_eq!(i2c.read_count(), 1);
        assert_eq!(i2c.last_read_addr(), Some(0x48));
        assert_eq!(i2c.last_read_len(), Some(4));
    }

    #[test]
    fn test_mock_i2c_write_read_combines_operations() {
        let mut i2c = MockI2c::new();
        let mut buffer = [0u8; 2];

        i2c.write_read(0x48, &[0x03], &mut buffer).unwrap();

        assert_eq!(buffer, [0xFF; 2]);
        assert_eq!(i2c.writes(), vec![(0x48, vec![0x03])]);
        assert_eq!(i2c.read_count(), 1);
        assert_eq!(i2c.last_read_addr(), Some(0x48));
        assert_eq!(i2c.last_read_len(), Some(2));
    }

    #[test]
    fn test_mock_i2c_clone_shares_state() {
        let mut i2c = MockI2c::new();
        let observer = i2c.clone();
        let mut buffer = [0u8; 1];

        i2c.read(0x20, &mut buffer).unwrap();

        assert_eq!(observer.read_count(), 1);
        assert_eq!(observer.last_read_addr(), Some(0x20));
        assert_eq!(observer.last_read_len(), Some(1));
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
}
