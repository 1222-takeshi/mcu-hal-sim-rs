// 動作確認用の簡単なテストコード
// このPRではモックHALのログ出力を確認するため、最小限の実装のみ
mod mock_hal;
use mock_hal::{MockPin, MockI2c};
use hal_api::gpio::OutputPin;
use hal_api::i2c::I2cBus;

fn main() {
    println!("=== Mock HAL Test ===");
    
    // MockPinのテスト
    let mut pin = MockPin::new(13);
    println!("\nTesting MockPin:");
    let _ = pin.set_high();
    let _ = pin.set_low();
    let _ = pin.set(true);
    let _ = pin.set(false);
    
    // MockI2cのテスト
    let mut i2c = MockI2c::new();
    println!("\nTesting MockI2c:");
    let _ = i2c.write(0x48, &[0x01, 0x02]);
    let mut buffer = [0u8; 4];
    let _ = i2c.read(0x48, &mut buffer);
    let _ = i2c.write_read(0x48, &[0x03], &mut buffer);
    
    println!("\n=== Test Complete ===");
}

