use hal_api::gpio::OutputPin;
use hal_api::i2c::I2cBus;

/// HALトレイトのみに依存するアプリケーションロジック
///
/// このアプリケーション構造体は、プラットフォーム固有の実装に依存せず、
/// HALトレイト（`OutputPin`と`I2cBus`）のみを使用します。
/// これにより、同じアプリケーションロジックをPCシミュレータ上でも
/// 実機（ESP32、Arduino Nano、Raspberry Pi Picoなど）上でも動作させることができます。
///
/// # ジェネリック型パラメータ
/// * `PIN` - GPIO出力ピンの実装（`OutputPin`トレイトを実装）
/// * `I2C` - I2Cバスの実装（`I2cBus`トレイトを実装）
pub struct App<PIN, I2C> {
    pin: PIN,
    i2c: I2C,
}

impl<PIN, I2C> App<PIN, I2C>
where
    PIN: OutputPin,
    I2C: I2cBus,
{
    /// 指定されたハードウェアインターフェースで新しいアプリケーションインスタンスを作成します。
    ///
    /// # 引数
    /// * `pin` - GPIO出力ピンの実装
    /// * `i2c` - I2Cバスの実装
    ///
    /// # 戻り値
    /// 新しい`App`インスタンス
    pub fn new(pin: PIN, i2c: I2C) -> Self {
        Self { pin, i2c }
    }

    /// アプリケーションロジックの1サイクルを実行します。
    ///
    /// この関数は定期的に呼び出され、アプリケーションの状態を更新します。
    /// 現在はプレースホルダー実装で、将来的にはLEDの点滅やセンサーの読み取りなどの
    /// 実際のロジックを実装予定です。
    pub fn tick(&mut self) {
        // TODO: アプリケーションロジックを実装
        // 例: LEDの点滅、I2Cセンサーからのデータ読み取りなど
        let _ = &mut self.pin;
        let _ = &mut self.i2c;
    }
}

