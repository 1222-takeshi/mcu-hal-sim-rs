/// デジタル出力ピンのトレイト
///
/// このトレイトは、デジタル出力ピンをHigh/Low状態に設定する機能を提供します。
/// マイコンのGPIOピンを抽象化し、プラットフォーム非依存なコードを実現します。
pub trait OutputPin {
    /// ピン操作時のエラー型
    type Error;

    /// ピンをHigh状態に設定します。
    ///
    /// # エラー
    /// ピンの状態を変更できない場合にエラーを返します。
    fn set_high(&mut self) -> Result<(), Self::Error>;

    /// ピンをLow状態に設定します。
    ///
    /// # エラー
    /// ピンの状態を変更できない場合にエラーを返します。
    fn set_low(&mut self) -> Result<(), Self::Error>;

    /// ピンを指定された状態に設定します。
    ///
    /// # 引数
    /// * `high` - `true`ならHigh状態、`false`ならLow状態に設定
    ///
    /// # エラー
    /// ピンの状態を変更できない場合にエラーを返します。
    fn set(&mut self, high: bool) -> Result<(), Self::Error> {
        if high {
            self.set_high()
        } else {
            self.set_low()
        }
    }
}

/// デジタル入力ピンのトレイト
///
/// このトレイトは、デジタル入力ピンの状態を読み取る機能を提供します。
/// マイコンのGPIOピンを抽象化し、プラットフォーム非依存なコードを実現します。
pub trait InputPin {
    /// ピン操作時のエラー型
    type Error;

    /// ピンがHigh状態かどうかを判定します。
    ///
    /// # 戻り値
    /// ピンがHigh状態の場合は`true`を返します。
    ///
    /// # エラー
    /// ピンの状態を読み取れない場合にエラーを返します。
    fn is_high(&self) -> Result<bool, Self::Error>;

    /// ピンがLow状態かどうかを判定します。
    ///
    /// # 戻り値
    /// ピンがLow状態の場合は`true`を返します。
    ///
    /// # エラー
    /// ピンの状態を読み取れない場合にエラーを返します。
    fn is_low(&self) -> Result<bool, Self::Error>;
}

