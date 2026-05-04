//! Host-side BH1750 照度センサモック

use crate::virtual_i2c::VirtualI2cDevice;
use hal_api::error::I2cError;
use hal_api::light::{LightReading, LightSensor};
use std::cell::RefCell;
use std::rc::Rc;
use std::vec::Vec;

#[derive(Debug)]
struct MockBh1750State {
    /// 順番に返す lux×100 値のシーケンス
    lux_x100_sequence: Vec<u32>,
    next_index: usize,
    loop_forever: bool,
    /// 受信したコマンドバイト
    last_command: Option<u8>,
}

#[derive(Clone, Debug)]
pub struct MockBh1750Device {
    state: Rc<RefCell<MockBh1750State>>,
}

impl MockBh1750Device {
    /// 固定の lux×100 シーケンスを返すモックを生成します。
    pub fn new(lux_x100_sequence: Vec<u32>) -> Self {
        Self {
            state: Rc::new(RefCell::new(MockBh1750State {
                lux_x100_sequence,
                next_index: 0,
                loop_forever: false,
                last_command: None,
            })),
        }
    }

    /// シーケンス末尾到達後もループするモックを生成します。
    pub fn looping(lux_x100_sequence: Vec<u32>) -> Self {
        let device = Self::new(lux_x100_sequence);
        device.state.borrow_mut().loop_forever = true;
        device
    }

    /// 固定照度値を返すシンプルなモックを生成します（lux×100）。
    pub fn fixed(lux_x100: u32) -> Self {
        Self::looping(vec![lux_x100])
    }
}

impl Default for MockBh1750Device {
    fn default() -> Self {
        // デフォルト: 10000 lux×100 = 100.00 lx（屋内照明相当）
        Self::fixed(10_000)
    }
}

impl VirtualI2cDevice for MockBh1750Device {
    fn write(&mut self, bytes: &[u8]) -> Result<(), I2cError> {
        if let Some(&cmd) = bytes.first() {
            self.state.borrow_mut().last_command = Some(cmd);
        }
        Ok(())
    }

    /// 2 バイトのセンサ生値を返します。
    /// `raw = lux_x100 * 120 / 100` の逆変換で raw を復元します。
    fn read(&mut self, buffer: &mut [u8]) -> Result<(), I2cError> {
        if buffer.len() < 2 {
            return Err(I2cError::BusError);
        }
        let mut state = self.state.borrow_mut();
        let lux_x100 = *state.lux_x100_sequence.get(state.next_index).unwrap_or(&0);

        // lux_x100 = raw * 500 / 6  →  raw = lux_x100 * 6 / 500
        let raw = (lux_x100 * 6 / 500) as u16;
        let [hi, lo] = raw.to_be_bytes();
        buffer[0] = hi;
        buffer[1] = lo;

        if state.loop_forever {
            state.next_index = (state.next_index + 1) % state.lux_x100_sequence.len();
        } else if state.next_index + 1 < state.lux_x100_sequence.len() {
            state.next_index += 1;
        }
        Ok(())
    }
}

/// `LightSensor` 直接実装（VirtualI2cBus を経由しない軽量版）
#[derive(Clone, Debug)]
pub struct MockLightSensor {
    device: MockBh1750Device,
}

impl MockLightSensor {
    pub fn fixed(lux_x100: u32) -> Self {
        Self {
            device: MockBh1750Device::fixed(lux_x100),
        }
    }

    pub fn looping(seq: Vec<u32>) -> Self {
        Self {
            device: MockBh1750Device::looping(seq),
        }
    }
}

impl Default for MockLightSensor {
    fn default() -> Self {
        Self::fixed(10_000)
    }
}

impl LightSensor for MockLightSensor {
    type Error = I2cError;

    fn read_lux(&mut self) -> Result<LightReading, I2cError> {
        let mut state = self.device.state.borrow_mut();
        let lux_x100 = *state.lux_x100_sequence.get(state.next_index).unwrap_or(&0);
        if state.loop_forever {
            state.next_index = (state.next_index + 1) % state.lux_x100_sequence.len();
        } else if state.next_index + 1 < state.lux_x100_sequence.len() {
            state.next_index += 1;
        }
        Ok(LightReading::new(lux_x100))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_bh1750_returns_correct_raw_bytes() {
        let mut device = MockBh1750Device::fixed(12_000); // 120.00 lx
        let mut buf = [0u8; 2];
        device.read(&mut buf).unwrap();
        // raw = 12000 * 6 / 500 = 144 = 0x0090
        assert_eq!(buf, [0x00, 0x90]);
    }

    #[test]
    fn mock_light_sensor_returns_fixed_lux() {
        let mut sensor = MockLightSensor::fixed(5000); // 50.00 lx
        let r = sensor.read_lux().unwrap();
        assert_eq!(r.lux_x100, 5000);
        assert_eq!(r.lux_integer(), 50);
    }

    #[test]
    fn mock_light_sensor_cycles_through_sequence() {
        let mut sensor = MockLightSensor::looping(vec![1000, 2000, 3000]);
        assert_eq!(sensor.read_lux().unwrap().lux_x100, 1000);
        assert_eq!(sensor.read_lux().unwrap().lux_x100, 2000);
        assert_eq!(sensor.read_lux().unwrap().lux_x100, 3000);
        assert_eq!(sensor.read_lux().unwrap().lux_x100, 1000); // ループ
    }
}
