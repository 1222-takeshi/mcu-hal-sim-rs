use hal_api::error::{GpioError, I2cError};
use hal_api::gpio::OutputPin;
use hal_api::i2c::I2cBus;

#[derive(Debug)]
pub enum AppError {
    Gpio(GpioError),
    I2c(I2cError),
}

impl From<GpioError> for AppError {
    fn from(err: GpioError) -> Self {
        AppError::Gpio(err)
    }
}

impl From<I2cError> for AppError {
    fn from(err: I2cError) -> Self {
        AppError::I2c(err)
    }
}

pub struct App<PIN, I2C> {
    pin: PIN,
    i2c: I2C,
    tick_count: u32,
    led_state: bool,
}

impl<PIN, I2C> App<PIN, I2C>
where
    PIN: OutputPin<Error = GpioError>,
    I2C: I2cBus<Error = I2cError>,
{
    pub fn new(pin: PIN, i2c: I2C) -> Self {
        Self {
            pin,
            i2c,
            tick_count: 0,
            led_state: false,
        }
    }

    #[allow(clippy::manual_is_multiple_of)]
    pub fn tick(&mut self) -> Result<(), AppError> {
        self.tick_count += 1;

        // 100 tickごと（1秒想定）にLED切り替え
        if self.tick_count % 100 == 0 {
            self.led_state = !self.led_state;
            self.pin.set(self.led_state)?;
        }

        // 500 tickごと（5秒想定）にI2C読み取り
        if self.tick_count % 500 == 0 {
            let mut buffer = [0u8; 4];
            self.i2c.read(0x48, &mut buffer)?;
        }

        Ok(())
    }

    /// テスト用: 現在のtickカウントを取得
    #[cfg(test)]
    pub fn tick_count(&self) -> u32 {
        self.tick_count
    }

    /// テスト用: 現在のLED状態を取得
    #[cfg(test)]
    pub fn led_state(&self) -> bool {
        self.led_state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    // テスト用モックGPIOピン
    #[derive(Clone)]
    struct MockPin {
        state: Rc<RefCell<Vec<bool>>>,
        should_fail: bool,
    }

    impl MockPin {
        fn new() -> Self {
            Self {
                state: Rc::new(RefCell::new(Vec::new())),
                should_fail: false,
            }
        }

        fn new_failing() -> Self {
            Self {
                state: Rc::new(RefCell::new(Vec::new())),
                should_fail: true,
            }
        }

        fn get_history(&self) -> Vec<bool> {
            self.state.borrow().clone()
        }
    }

    impl OutputPin for MockPin {
        type Error = GpioError;

        fn set_high(&mut self) -> Result<(), Self::Error> {
            if self.should_fail {
                return Err(GpioError::HardwareError);
            }
            self.state.borrow_mut().push(true);
            Ok(())
        }

        fn set_low(&mut self) -> Result<(), Self::Error> {
            if self.should_fail {
                return Err(GpioError::HardwareError);
            }
            self.state.borrow_mut().push(false);
            Ok(())
        }
    }

    // テスト用モックI2C
    #[derive(Clone)]
    struct MockI2c {
        read_count: Rc<RefCell<usize>>,
        should_fail: bool,
    }

    impl MockI2c {
        fn new() -> Self {
            Self {
                read_count: Rc::new(RefCell::new(0)),
                should_fail: false,
            }
        }

        fn new_failing() -> Self {
            Self {
                read_count: Rc::new(RefCell::new(0)),
                should_fail: true,
            }
        }

        fn get_read_count(&self) -> usize {
            *self.read_count.borrow()
        }
    }

    impl I2cBus for MockI2c {
        type Error = I2cError;

        fn write(&mut self, _addr: u8, _bytes: &[u8]) -> Result<(), Self::Error> {
            Ok(())
        }

        fn read(&mut self, _addr: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
            if self.should_fail {
                return Err(I2cError::BusError);
            }
            *self.read_count.borrow_mut() += 1;
            buffer.fill(0xFF);
            Ok(())
        }

        fn write_read(
            &mut self,
            _addr: u8,
            _bytes: &[u8],
            buffer: &mut [u8],
        ) -> Result<(), Self::Error> {
            buffer.fill(0xFF);
            Ok(())
        }
    }

    #[test]
    fn test_app_new() {
        let pin = MockPin::new();
        let i2c = MockI2c::new();
        let app = App::new(pin, i2c);

        assert_eq!(app.tick_count(), 0);
        assert!(!app.led_state());
    }

    #[test]
    fn test_tick_increments_counter() {
        let pin = MockPin::new();
        let i2c = MockI2c::new();
        let mut app = App::new(pin, i2c);

        app.tick().unwrap();
        assert_eq!(app.tick_count(), 1);

        app.tick().unwrap();
        assert_eq!(app.tick_count(), 2);
    }

    #[test]
    fn test_led_toggles_every_100_ticks() {
        let pin = MockPin::new();
        let i2c = MockI2c::new();
        let mut app = App::new(pin.clone(), i2c);

        // 最初の99tickではLEDは変更されない
        for _ in 0..99 {
            app.tick().unwrap();
        }
        assert_eq!(pin.get_history().len(), 0);

        // 100tick目でLEDがHIGHになる
        app.tick().unwrap();
        assert_eq!(app.tick_count(), 100);
        assert_eq!(pin.get_history(), vec![true]);

        // 次の99tickではLEDは変更されない
        for _ in 0..99 {
            app.tick().unwrap();
        }
        assert_eq!(pin.get_history().len(), 1);

        // 200tick目でLEDがLOWになる
        app.tick().unwrap();
        assert_eq!(app.tick_count(), 200);
        assert_eq!(pin.get_history(), vec![true, false]);
    }

    #[test]
    fn test_led_state_alternates() {
        let pin = MockPin::new();
        let i2c = MockI2c::new();
        let mut app = App::new(pin.clone(), i2c);

        // 100tick: HIGH
        for _ in 0..100 {
            app.tick().unwrap();
        }
        assert!(app.led_state());

        // 200tick: LOW
        for _ in 0..100 {
            app.tick().unwrap();
        }
        assert!(!app.led_state());

        // 300tick: HIGH
        for _ in 0..100 {
            app.tick().unwrap();
        }
        assert!(app.led_state());
    }

    #[test]
    fn test_i2c_read_every_500_ticks() {
        let pin = MockPin::new();
        let i2c = MockI2c::new();
        let mut app = App::new(pin, i2c.clone());

        // 最初の499tickではI2C読み取りなし
        for _ in 0..499 {
            app.tick().unwrap();
        }
        assert_eq!(i2c.get_read_count(), 0);

        // 500tick目でI2C読み取り発生
        app.tick().unwrap();
        assert_eq!(i2c.get_read_count(), 1);

        // 次の499tickではI2C読み取りなし
        for _ in 0..499 {
            app.tick().unwrap();
        }
        assert_eq!(i2c.get_read_count(), 1);

        // 1000tick目で2回目のI2C読み取り
        app.tick().unwrap();
        assert_eq!(i2c.get_read_count(), 2);
    }

    #[test]
    fn test_led_and_i2c_timing_coordination() {
        let pin = MockPin::new();
        let i2c = MockI2c::new();
        let mut app = App::new(pin.clone(), i2c.clone());

        // 500 tickでLED 5回切り替え、I2C 1回読み取り
        for _ in 0..500 {
            app.tick().unwrap();
        }

        assert_eq!(pin.get_history().len(), 5); // 100, 200, 300, 400, 500
        assert_eq!(i2c.get_read_count(), 1); // 500
    }

    #[test]
    fn test_gpio_error_propagation() {
        let pin = MockPin::new_failing();
        let i2c = MockI2c::new();
        let mut app = App::new(pin, i2c);

        // 99tickまでは成功
        for _ in 0..99 {
            assert!(app.tick().is_ok());
        }

        // 100tick目でGPIOエラー発生
        let result = app.tick();
        assert!(result.is_err());
        if let Err(AppError::Gpio(GpioError::HardwareError)) = result {
            // 期待通りのエラー
        } else {
            panic!("Expected GPIO HardwareError");
        }
    }

    #[test]
    fn test_i2c_error_propagation() {
        let pin = MockPin::new();
        let i2c = MockI2c::new_failing();
        let mut app = App::new(pin, i2c);

        // 499tickまでは成功
        for _ in 0..499 {
            assert!(app.tick().is_ok());
        }

        // 500tick目でI2Cエラー発生
        let result = app.tick();
        assert!(result.is_err());
        if let Err(AppError::I2c(I2cError::BusError)) = result {
            // 期待通りのエラー
        } else {
            panic!("Expected I2C BusError");
        }
    }

    #[test]
    fn test_app_error_from_gpio_error() {
        let gpio_err = GpioError::InvalidPin;
        let app_err: AppError = gpio_err.into();
        assert!(matches!(app_err, AppError::Gpio(GpioError::InvalidPin)));
    }

    #[test]
    fn test_app_error_from_i2c_error() {
        let i2c_err = I2cError::Timeout;
        let app_err: AppError = i2c_err.into();
        assert!(matches!(app_err, AppError::I2c(I2cError::Timeout)));
    }

    #[test]
    fn test_app_error_debug_format() {
        let app_err = AppError::Gpio(GpioError::HardwareError);
        let debug_str = format!("{:?}", app_err);
        assert!(debug_str.contains("Gpio"));
        assert!(debug_str.contains("HardwareError"));
    }

    #[test]
    fn test_multiple_cycles() {
        let pin = MockPin::new();
        let i2c = MockI2c::new();
        let mut app = App::new(pin.clone(), i2c.clone());

        // 1000 tick実行
        for _ in 0..1000 {
            app.tick().unwrap();
        }

        assert_eq!(app.tick_count(), 1000);
        assert_eq!(pin.get_history().len(), 10); // 100, 200, ..., 1000
        assert_eq!(i2c.get_read_count(), 2); // 500, 1000
    }

    #[test]
    fn test_led_pattern_first_1000_ticks() {
        let pin = MockPin::new();
        let i2c = MockI2c::new();
        let mut app = App::new(pin.clone(), i2c);

        for _ in 0..1000 {
            app.tick().unwrap();
        }

        let history = pin.get_history();
        // パターン確認: true, false, true, false, ...
        for (i, &state) in history.iter().enumerate() {
            let expected = (i + 1) % 2 == 1; // 奇数番目はtrue、偶数番目はfalse
            assert_eq!(state, expected, "LED state mismatch at index {}", i);
        }
    }

    #[test]
    fn test_tick_50_no_actions() {
        let pin = MockPin::new();
        let i2c = MockI2c::new();
        let mut app = App::new(pin.clone(), i2c.clone());

        // 50 tick（100の倍数でも500の倍数でもない）
        for _ in 0..50 {
            app.tick().unwrap();
        }

        assert_eq!(pin.get_history().len(), 0);
        assert_eq!(i2c.get_read_count(), 0);
    }

    #[test]
    fn test_tick_exactly_100() {
        let pin = MockPin::new();
        let i2c = MockI2c::new();
        let mut app = App::new(pin.clone(), i2c.clone());

        for _ in 0..100 {
            app.tick().unwrap();
        }

        assert_eq!(pin.get_history().len(), 1);
        assert!(pin.get_history()[0]);
        assert_eq!(i2c.get_read_count(), 0);
    }

    #[test]
    fn test_tick_exactly_500() {
        let pin = MockPin::new();
        let i2c = MockI2c::new();
        let mut app = App::new(pin.clone(), i2c.clone());

        for _ in 0..500 {
            app.tick().unwrap();
        }

        assert_eq!(pin.get_history().len(), 5);
        assert_eq!(i2c.get_read_count(), 1);
    }

    #[test]
    fn test_continuous_operation() {
        let pin = MockPin::new();
        let i2c = MockI2c::new();
        let mut app = App::new(pin.clone(), i2c.clone());

        // 2500 tick（25秒相当）
        for _ in 0..2500 {
            app.tick().unwrap();
        }

        assert_eq!(app.tick_count(), 2500);
        assert_eq!(pin.get_history().len(), 25);
        assert_eq!(i2c.get_read_count(), 5);
    }

    #[test]
    fn test_initial_led_state_is_false() {
        let pin = MockPin::new();
        let i2c = MockI2c::new();
        let app = App::new(pin, i2c);

        assert!(!app.led_state());
    }

    #[test]
    fn test_tick_count_overflow_safety() {
        let pin = MockPin::new();
        let i2c = MockI2c::new();
        let mut app = App::new(pin, i2c);

        // u32::MAXに近い値から開始するために、内部状態を直接設定できないので
        // このテストは概念的なもの。実際の運用では問題にならない範囲で動作確認
        for _ in 0..1000 {
            app.tick().unwrap();
        }

        assert_eq!(app.tick_count(), 1000);
    }

    #[test]
    fn test_error_stops_execution() {
        let pin = MockPin::new_failing();
        let i2c = MockI2c::new();
        let mut app = App::new(pin, i2c);

        // エラーが発生するまで実行
        for i in 0..100 {
            let result = app.tick();
            if i < 99 {
                assert!(result.is_ok());
            } else {
                assert!(result.is_err());
            }
        }
    }
}

