# mcu-hal-sim-rs 開発プラン

## プロジェクト概要

ESP32/Arduino Nano/Raspberry Pi Pico 等のマイコン向けRustアプリケーションを、MCU非依存のHAL trait経由で記述し、PC上のシミュレータで動作確認できるようにする。将来的にはESP32実機向けバイナリもビルド可能とする。

---

## 現状分析（2025-12-21時点）

### 完了済み ✅
- [x] Cargo workspace構成の初期化
  - `crates/hal-api`: HAL trait定義（GPIO/I2C）
  - `crates/core-app`: アプリロジック（`App<PIN, I2C>`構造体）
  - `crates/platform-pc-sim`: PC用バイナリクレート（最小限のmain関数のみ）
- [x] HAL trait定義
  - `OutputPin` / `InputPin`: GPIO操作
  - `I2cBus`: I2C通信（write/read/write_read）
  - 各traitに`type Error`を持ち、`Result<_, Self::Error>`を返す設計
- [x] `core-app`の`App`構造体
  - ジェネリクスで`PIN: OutputPin`, `I2C: I2cBus`を受け取る
  - `new(pin, i2c)`, `tick(&mut self)`メソッド
  - **現状**: `tick()`はプレースホルダー（TODO未実装）
- [x] プロジェクトドキュメント整備
  - `.codex/AGENTS.md`: Codex用エージェント定義
  - `.github/copilot-instructions.md`: Copilot向けコンテキスト
  - `tmp/pr-init-hal-workspace.md`: PR#1の説明文草案
  - `tmp/project-context.md`: プロジェクトコンテキスト

### 未完了・課題 ⚠️
- [ ] **PCシミュレータのHAL実装がない**
  - `platform-pc-sim/main.rs`は`println!("pc sim");`のみ
  - `OutputPin` / `InputPin` / `I2cBus`の具体的なモック実装が必要
- [ ] **`core-app`のアプリロジックがプレースホルダー**
  - `tick()`メソッドが実質的な処理を何も行っていない
  - 最初の目標（LED点滅、ダミーI2C読み取り）が未実装
- [ ] **テストが存在しない**
  - ユニットテスト、統合テストが一切ない
  - ビルド確認も未実施（Codex環境制約により）
- [ ] **エラー型の設計が未定義**
  - 各traitの`type Error`が関連型として宣言されているが、具体的な型定義がない
- [ ] **READMEが空**
  - プロジェクト説明、ビルド手順、使い方の記載がない
- [ ] **examples/ディレクトリが存在しない**
  - LED点滅、I2Cセンサ読み取りなどのサンプルコードがない
- [ ] **ESP32実機向けクレートがない**
  - `crates/platform-esp32`の追加は後回し（PC版安定後）

---

## 開発の優先順位と段階的アプローチ（ESP32適合重視版）

### 戦略: 早期にESP32で検証し、実機フィードバックを得る 🚀

**基本方針**: 
1. **最小限のモック実装**でPCシミュレータの骨格を作る
2. **早期にESP32実機対応**を開始し、HAL設計の妥当性を検証
3. 実機で得た知見をPCシミュレータにフィードバック
4. 両プラットフォームを並行して充実させる

この戦略により、以下のリスクを早期に発見・対処できます：
- HAL traitの設計が実機要件に合わない
- エラー型のマッピングが困難
- 実機特有の制約（メモリ、リアルタイム性）への対応

---

### フェーズ1: 最小限のモック実装とESP32検証準備 🎯

**目標**: 
- PCシミュレータの最小限の動作確認
- ESP32実機対応のための基盤整備
- 両プラットフォームで共通のアプリロジックが動くことを確認

#### 1.1 エラー型の設計と実装
- **タスク**: `hal-api`にシンプルなエラー型を追加
- **実装方針**:
  ```rust
  // hal-api/error.rs
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub enum HalError {
      Gpio(GpioError),
      I2c(I2cError),
  }
  
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub enum GpioError {
      InvalidPin,
      HardwareError,
  }
  
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub enum I2cError {
      InvalidAddress,
      BusError,
      Timeout,
  }
  ```
- **受け入れ基準**:
  - [x] `hal-api/error.rs`が作成され、`lib.rs`でエクスポート
  - [x] 各traitの実装でエラー型が利用可能

#### 1.2 PCシミュレータのHAL実装（モック）
- **タスク**: `platform-pc-sim`にモックHAL実装を追加
- **実装方針**:
  ```rust
  // platform-pc-sim/mock_hal.rs
  pub struct MockPin {
      pin_number: u8,
      state: bool,
  }
  
  impl OutputPin for MockPin {
      type Error = hal_api::error::GpioError;
      
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
  
  impl I2cBus for MockI2c {
      type Error = hal_api::error::I2cError;
      
      fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
          println!("[I2C] Write to 0x{:02X}: {:?}", addr, bytes);
          Ok(())
      }
      
      fn read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
          println!("[I2C] Read from 0x{:02X}: {} bytes", addr, buffer.len());
          // ダミーデータで埋める
          buffer.fill(0xFF);
          Ok(())
      }
      
      fn write_read(&mut self, addr: u8, bytes: &[u8], buffer: &mut [u8]) -> Result<(), Self::Error> {
          self.write(addr, bytes)?;
          self.read(addr, buffer)
      }
  }
  ```
- **受け入れ基準**:
  - [x] `platform-pc-sim/mock_hal.rs`が実装される
  - [x] `MockPin`が`OutputPin`を実装
  - [x] `MockI2c`が`I2cBus`を実装
  - [x] GPIO/I2C操作が標準出力にログ出力される

#### 1.3 `core-app`のアプリロジック実装
- **タスク**: `App::tick()`に具体的な処理を実装
- **実装方針**:
  ```rust
  pub struct App<PIN, I2C> {
      pin: PIN,
      i2c: I2C,
      tick_count: u32,
      led_state: bool,
  }
  
  impl<PIN, I2C> App<PIN, I2C>
  where
      PIN: OutputPin,
      I2C: I2cBus,
  {
      pub fn new(pin: PIN, i2c: I2C) -> Self {
          Self {
              pin,
              i2c,
              tick_count: 0,
              led_state: false,
          }
      }
      
      pub fn tick(&mut self) -> Result<(), AppError> {
          self.tick_count += 1;
          
          // 1秒ごと（100 tick想定）にLED切り替え
          if self.tick_count % 100 == 0 {
              self.led_state = !self.led_state;
              self.pin.set(self.led_state)?;
          }
          
          // 5秒ごとにダミーI2C読み取り
          if self.tick_count % 500 == 0 {
              let mut buffer = [0u8; 4];
              self.i2c.read(0x48, &mut buffer)?;
          }
          
          Ok(())
      }
  }
  ```
- **受け入れ基準**:
  - [x] `App::tick()`がLED点滅ロジックを実装
  - [x] 定期的にI2C読み取りを実行
  - [x] エラーハンドリングが適切

#### 1.4 `platform-pc-sim`のメインループ実装
- **タスク**: `main.rs`でアプリを初期化し、ループ実行
- **実装方針**:
  ```rust
  use core_app::App;
  use hal_api::gpio::OutputPin;
  use hal_api::i2c::I2cBus;
  mod mock_hal;
  use mock_hal::{MockPin, MockI2c};
  use std::thread;
  use std::time::Duration;
  
  fn main() {
      println!("=== PC Simulator Started ===");
      
      let pin = MockPin::new(13);  // GPIO13をLEDに見立てる
      let i2c = MockI2c::new();
      
      let mut app = App::new(pin, i2c);
      
      loop {
          if let Err(e) = app.tick() {
              eprintln!("Error: {:?}", e);
              break;
          }
          thread::sleep(Duration::from_millis(10));  // 10ms = 100Hz
      }
  }
  ```
- **受け入れ基準**:
  - [x] PCシミュレータが起動し、無限ループでtick()を呼ぶ
  - [x] Ctrl+Cで終了可能
  - [x] 標準出力にGPIO/I2Cログが出力される

#### 1.5 PCシミュレータの動作確認
- **タスク**: 最小限のテストと動作確認
- **対象**:
  - `cargo build`の成功確認
  - `cargo run -p platform-pc-sim`での起動確認
  - 基本的なログ出力の確認
- **受け入れ基準**:
  - [x] ビルドが成功
  - [x] PCシミュレータが起動し、LED点滅・I2Cログが出力される
  - [x] 基本的な動作が目視確認できる

---

### フェーズ2: ESP32実機対応（最重要検証）🔥

**目標**: HAL設計の妥当性を実機で検証し、必要な修正を早期に発見

#### 2.1 ESP32開発環境のセットアップ
- **タスク**: ESP32向けRust開発環境の構築
- **手順**:
  ```bash
  # espup のインストール（ESP32 Rust ツールチェーン管理）
  cargo install espup
  espup install
  
  # espflash のインストール（書き込みツール）
  cargo install espflash
  
  # cargo-generate のインストール（テンプレート利用）
  cargo install cargo-generate
  ```
- **確認事項**:
  - ESP32ボードの接続確認
  - シリアルポートの認識確認
  - 簡単なLチカプログラムで動作確認
- **受け入れ基準**:
  - [x] ESP32向けビルド環境が整う
  - [x] サンプルプログラムが実機で動作する

#### 2.2 `crates/platform-esp32`の作成
- **タスク**: ESP32用プラットフォームクレートの追加
- **構成**:
  ```
  crates/platform-esp32/
  ├── Cargo.toml          # esp-hal等の依存関係
  ├── .cargo/config.toml  # ESP32向けビルド設定
  ├── rust-toolchain.toml # ツールチェーン指定
  ├── main.rs             # エントリポイント
  └── esp32_hal.rs        # HAL trait実装
  ```
- **依存クレート**:
  ```toml
  [dependencies]
  hal-api = { path = "../hal-api" }
  core-app = { path = "../core-app" }
  esp-hal = "0.20"              # ESP32 HAL
  esp-backtrace = "0.14"        # パニックハンドラ
  esp-println = "0.11"          # シリアル出力
  ```
- **受け入れ基準**:
  - [x] クレート構造が作成される
  - [x] 依存関係が適切に設定される

#### 2.3 ESP32用HAL実装（GPIO）
- **タスク**: `OutputPin`の実装
- **実装例**:
  ```rust
  use hal_api::gpio::OutputPin;
  use esp_hal::gpio::{Output, PushPull, GpioPin};
  
  pub struct Esp32OutputPin<const PIN: u8> {
      pin: Output<'static, GpioPin<PIN>>,
  }
  
  impl<const PIN: u8> OutputPin for Esp32OutputPin<PIN> {
      type Error = hal_api::error::GpioError;
      
      fn set_high(&mut self) -> Result<(), Self::Error> {
          self.pin.set_high();
          Ok(())
      }
      
      fn set_low(&mut self) -> Result<(), Self::Error> {
          self.pin.set_low();
          Ok(())
      }
  }
  ```
- **検証ポイント**:
  - ESP32のGPIOピン制御がHAL trait経由で動作するか
  - エラー型のマッピングが適切か
  - ライフタイム管理が問題ないか
- **受け入れ基準**:
  - [x] `OutputPin` trait実装が完了
  - [x] 実機でLED点滅が動作

#### 2.4 ESP32用HAL実装（I2C）
- **タスク**: `I2cBus`の実装
- **実装例**:
  ```rust
  use hal_api::i2c::I2cBus;
  use esp_hal::i2c::I2C;
  
  pub struct Esp32I2c<'d> {
      i2c: I2C<'d, esp_hal::peripherals::I2C0>,
  }
  
  impl<'d> I2cBus for Esp32I2c<'d> {
      type Error = hal_api::error::I2cError;
      
      fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
          self.i2c.write(addr, bytes)
              .map_err(|_| hal_api::error::I2cError::BusError)
      }
      
      fn read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
          self.i2c.read(addr, buffer)
              .map_err(|_| hal_api::error::I2cError::BusError)
      }
      
      fn write_read(&mut self, addr: u8, bytes: &[u8], buffer: &mut [u8]) 
          -> Result<(), Self::Error> {
          self.i2c.write_read(addr, bytes, buffer)
              .map_err(|_| hal_api::error::I2cError::BusError)
      }
  }
  ```
- **検証ポイント**:
  - I2Cデバイス（BME280等のセンサ）との通信
  - エラーハンドリングの妥当性
  - タイムアウト処理の必要性
- **受け入れ基準**:
  - [x] `I2cBus` trait実装が完了
  - [x] 実機でI2Cセンサからデータ取得可能

#### 2.5 ESP32でのアプリ統合
- **タスク**: `core-app`のAppをESP32で実行
- **実装例**:
  ```rust
  #![no_std]
  #![no_main]
  
  use esp_hal::{
      clock::ClockControl,
      gpio::IO,
      i2c::I2C,
      peripherals::Peripherals,
      prelude::*,
      timer::TimerGroup,
      Delay,
  };
  use esp_backtrace as _;
  use esp_println::println;
  
  mod esp32_hal;
  use esp32_hal::{Esp32OutputPin, Esp32I2c};
  use core_app::App;
  
  #[entry]
  fn main() -> ! {
      let peripherals = Peripherals::take();
      let system = peripherals.SYSTEM.split();
      let clocks = ClockControl::boot_defaults(system.clock_control).freeze();
      
      let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
      let led_pin = Esp32OutputPin::new(io.pins.gpio2.into_push_pull_output());
      
      let i2c = I2C::new(
          peripherals.I2C0,
          io.pins.gpio21,  // SDA
          io.pins.gpio22,  // SCL
          100u32.kHz(),
          &clocks,
      );
      let i2c_bus = Esp32I2c::new(i2c);
      
      let mut app = App::new(led_pin, i2c_bus);
      let mut delay = Delay::new(&clocks);
      
      println!("=== ESP32 App Started ===");
      
      loop {
          if let Err(e) = app.tick() {
              println!("Error: {:?}", e);
          }
          delay.delay_millis(10);
      }
  }
  ```
- **検証ポイント**:
  - `no_std`環境での動作
  - メモリ使用量（ESP32の制約内か）
  - リアルタイム性能
  - パニック時の挙動
- **受け入れ基準**:
  - [x] ESP32でアプリが起動
  - [x] LED点滅とI2C通信が実機で動作
  - [x] シリアルモニタでログ確認可能

#### 2.6 実機検証で得た知見の整理
- **タスク**: HAL設計の改善点を文書化
- **確認事項**:
  - [ ] HAL traitのシグネチャは適切だったか？
  - [ ] エラー型の粒度は十分か？
  - [ ] ライフタイム管理で問題はないか？
  - [ ] 非同期処理の必要性は？
  - [ ] `no_std`制約で困った点は？
- **成果物**: `docs/esp32-integration-findings.md`
- **受け入れ基準**:
  - [ ] 改善点リストが文書化される
  - [ ] 必要に応じてHAL trait設計を修正

---

### フェーズ3: 実機フィードバックを反映した改善

**目標**: ESP32で得た知見をPCシミュレータと共通基盤に反映

#### 3.1 HAL trait設計の見直し（必要に応じて）
- **タスク**: 実機検証で判明した問題点の修正
- **想定される修正例**:
  - エラー型の追加（Timeout, NotReady等）
  - ライフタイム制約の緩和
  - 非同期版traitの追加（`async-trait`）
  - `no_std`対応の強化
- **受け入れ基準**:
  - [x] 修正がPC/ESP32両方で動作確認される
  - [x] 既存コードへの影響が最小限

#### 3.2 PCシミュレータの強化
- **タスク**: 実機の挙動をより忠実にシミュレート
- **改善例**:
  - GPIO状態の永続化（ファイル等）
  - I2Cデバイスのモック追加（BME280等）
  - エラー注入機能（テスト用）
  - タイミング制約のシミュレーション
- **受け入れ基準**:
  - [x] PCシミュレータで実機に近い動作確認が可能

#### 3.3 ユニットテストと統合テストの充実
- **タスク**: 包括的なテストスイートの構築
- **対象**:
  - `hal-api`: traitの動作テスト
  - `core-app`: アプリロジックのテスト
  - `platform-pc-sim`: モックHALのテスト
  - `platform-esp32`: （可能な範囲で）実機テスト
- **受け入れ基準**:
  - [x] `cargo test`が全クレートで成功
  - [x] テストカバレッジ80%以上

#### 3.4 README.mdの充実
- **内容**:
  - プロジェクト概要
  - アーキテクチャ図（Mermaid等）
  - **両プラットフォームのビルド・実行手順**
    - PCシミュレータ: `cargo run -p platform-pc-sim`
    - ESP32実機: `cargo espflash flash -p platform-esp32 --monitor`
  - 各クレートの役割説明
  - ESP32実機対応の完了報告
  - ハードウェア要件（ESP32ボード、配線図）
- **受け入れ基準**:
  - [x] READMEを読めばプロジェクトの全体像が理解できる
  - [x] PC/ESP32両方の環境構築手順が明確
  - [x] 配線図等のハードウェア情報が含まれる

#### 3.5 examples/ディレクトリの追加
- **サンプル**:
  - `examples/pc-sim/blink.rs`: PCシミュレータ用LED点滅
  - `examples/pc-sim/i2c_sensor.rs`: PCシミュレータ用I2C
  - `examples/esp32/blink.rs`: ESP32実機用LED点滅
  - `examples/esp32/i2c_bme280.rs`: ESP32実機用BME280センサ
- **受け入れ基準**:
  - [x] PCシミュレータ例が動作
  - [x] ESP32実機例が動作
  - [x] 各サンプルに詳細なコメント

#### 3.6 各クレートのドキュメントコメント追加
- **対象**: 全パブリックAPI
- **形式**: Rustdoc形式
- **受け入れ基準**:
  - [x] `cargo doc --open`でドキュメント生成・閲覧可能
  - [x] 主要なtraitとメソッドに使用例を記載

---

### フェーズ4: さらなる充実と他MCU対応

#### 4.1 CI/CDの整備
- **GitHub Actions**: 自動ビルド・テスト
- **マトリクスビルド**: PC/ESP32両方のビルド確認
- **クロスコンパイル**: ESP32向けバイナリのビルド確認
- **リリース自動化**: バイナリのアーティファクト生成

#### 4.2 他MCU対応の検討
- **Arduino Nano (AVR)**
  - `crates/platform-avr`
  - `avr-hal`の利用
- **Raspberry Pi Pico (RP2040)**
  - `crates/platform-rp2040`
  - `rp2040-hal`の利用
- **受け入れ基準**:
  - [x] 同じ`core-app`が複数MCUで動作
  - [x] HAL traitの汎用性が証明される

#### 4.3 高度な機能の追加
- **SPI / ADC / Timer**: 新しいHAL traitの追加
- **DMA対応**: 高速データ転送
- **割り込み処理**: イベント駆動アーキテクチャ
- **低消費電力モード**: スリープ制御

---

## タスク優先度マトリクス（ESP32早期検証版）

| フェーズ | タスク | 優先度 | 難易度 | 依存関係 | 期待される成果 |
|---------|--------|--------|--------|----------|----------------|
| **1** | 1.1 エラー型設計 | 🔥最高 | 低 | なし | 共通エラー型の確立 |
| **1** | 1.2 モックHAL実装 | 🔥最高 | 低 | 1.1 | PC動作確認の基盤 |
| **1** | 1.3 アプリロジック実装 | 🔥最高 | 中 | 1.1 | 共通ビジネスロジック |
| **1** | 1.4 メインループ実装 | 🔥最高 | 低 | 1.2, 1.3 | PCシミュレータ完成 |
| **1** | 1.5 PC動作確認 | 🔥最高 | 低 | 1.4 | 基本動作の検証 |
| **2** | 2.1 ESP32環境構築 | 🔥最高 | 中 | フェーズ1 | 実機開発準備 |
| **2** | 2.2 ESP32クレート作成 | 🔥最高 | 中 | 2.1 | 実機HAL基盤 |
| **2** | 2.3 ESP32 GPIO実装 | 🔥最高 | 中 | 2.2 | LED制御検証 |
| **2** | 2.4 ESP32 I2C実装 | 🔥最高 | 高 | 2.3 | センサ通信検証 |
| **2** | 2.5 ESP32統合実行 | 🔥最高 | 高 | 2.4 | **HAL設計の妥当性確認** |
| **2** | 2.6 知見の整理 | 🔥高 | 低 | 2.5 | 改善点の文書化 |
| **3** | 3.1 HAL再設計 | 高 | 中 | 2.6 | 設計品質向上 |
| **3** | 3.2 PCシミュレータ強化 | 高 | 中 | 3.1 | 実機挙動の再現 |
| **3** | 3.3 テスト充実 | 高 | 中 | 3.1, 3.2 | 品質保証 |
| **3** | 3.4 README充実 | 中 | 低 | フェーズ2完了 | ドキュメント整備 |
| **3** | 3.5 examples追加 | 中 | 低 | フェーズ2完了 | 学習資料 |
| **3** | 3.6 ドキュメントコメント | 中 | 低 | フェーズ2完了 | API説明 |
| **4** | 4.1 CI/CD整備 | 中 | 中 | フェーズ3完了 | 自動化 |
| **4** | 4.2 他MCU対応 | 低 | 高 | フェーズ3完了 | 汎用性証明 |
| **4** | 4.3 高度な機能 | 低 | 高 | 4.2 | 機能拡張 |

---

## 開発ガイドライン（再確認）

### Git/GitHub運用
- **ブランチ戦略**: `feat/<機能名>` / `fix/<修正内容>`
- **mainへの直接push禁止**: 必ずPR経由
- **PRルール**:
  - タイトル: 英語（Conventional Commits）
  - 本文: 日本語で詳細説明
  - テスト実行方法を明記
- **Issue駆動開発**: 作業はIssueベースで進める

### コーディングルール
- **シンプル・モダン**: 読みやすく、最新の言語機能を活用
- **小さな変更**: 1PR1機能、レビュー容易性を優先
- **TDD**: テストファースト、Red-Green-Refactor
- **エラーハンドリング**: Result型を適切に使用

### ツール・スクリプト
- **ghコマンド優先**: `git`より`gh`を使用
- **`scripts/gh-workflow.sh`**: push/pr/issue操作の共通化

---

## 次のアクション（推奨実行順序）

### 🚀 ステップ1: 最小限のPC実装（1-2日）
1. **Issue作成**: フェーズ1タスクをGitHub Issueとして登録
2. **エラー型実装**: `hal-api/error.rs`の作成
3. **簡易モックHAL**: `platform-pc-sim/mock_hal.rs`の基本実装
4. **簡易アプリロジック**: `core-app/lib.rs`のtick()に最小限の処理
5. **PC動作確認**: `cargo run -p platform-pc-sim`で動作確認
6. **PR作成**: フェーズ1完了をPR

### 🔥 ステップ2: ESP32実機検証（2-3日）
7. **ESP32環境構築**: `espup`, `espflash`のインストール
8. **ESP32クレート作成**: `crates/platform-esp32`の初期構造
9. **ESP32 GPIO実装**: LEDチカチカをESP32で実現
10. **ESP32 I2C実装**: BME280等のセンサと通信
11. **実機動作確認**: ESP32ボードで`core-app`を実行
12. **問題点の記録**: HAL設計の課題を文書化
13. **PR作成**: ESP32対応をPR

### 🔧 ステップ3: フィードバック反映（1-2日）
14. **HAL修正**: 実機で判明した問題を修正
15. **PCシミュレータ改善**: 実機挙動を反映
16. **テスト追加**: 包括的なテストスイート
17. **ドキュメント整備**: README、examples、Rustdoc
18. **PR作成**: 改善をPR

---

## 成功基準（ESP32重視版）

### ✅ フェーズ1完了時（PC基本動作）
- [ ] `cargo build`が全クレートで成功
- [ ] `cargo run -p platform-pc-sim`でPCシミュレータが起動
- [ ] 標準出力にLED点滅・I2C操作のログが出力される
- [ ] Ctrl+Cで正常終了できる
- [ ] **この時点でPRをマージ**

### 🔥 フェーズ2完了時（ESP32実機検証）**← 最重要マイルストーン**
- ✅ ESP32開発環境が構築され、サンプルが動作
- ✅ `cargo espflash flash -p platform-esp32 --monitor`でビルド・書き込み成功
- ✅ **ESP32実機でLEDが点滅する**
- ✅ **ESP32実機でI2Cセンサからデータ取得できる**
- ✅ `core-app`のアプリロジックがPC/ESP32両方で動作
- ✅ HAL設計の妥当性が実機で確認される
- ✅ 改善が必要な点が明確に文書化される
- ✅ **この時点でPRをマージし、大きなマイルストーン達成**

### 🔧 フェーズ3完了時（品質向上）
- ✅ 実機フィードバックを反映したHAL改善
- ✅ `cargo test`が全テストでpass
- ✅ READMEでPC/ESP32両環境の構築手順が明確
- ✅ サンプルコードが両プラットフォームで動作
- ✅ Rustdocドキュメントが生成・参照可能

### 🚀 フェーズ4完了時（さらなる充実）
- ✅ CI/CDで両プラットフォームの自動ビルド
- ✅ 他MCU対応の道筋が見える
- ✅ プロジェクトが成熟し、他者が貢献しやすい状態

---

## 補足: 技術的検討事項とESP32特有の考慮点

### エラー型の選択肢
- **Option 1**: 各trait実装ごとに独自のError型（シンプル、最初に採用）
- **Option 2**: `thiserror`クレートを使った統一Error型（`no_std`対応必要）
- **Option 3**: `anyhow`でシンプルなエラー伝播（標準ライブラリ依存、ESP32では不可）
- **推奨**: ESP32を考慮し、`no_std`互換のOption 1から始める

### ESP32特有の考慮事項

#### メモリ制約
- **SRAM**: ESP32は約320KB（モデルにより異なる）
- **対策**: 
  - スタック使用量の最小化
  - ヒープアロケーションの削減
  - `Box`、`Vec`の使用を慎重に
  - `static`変数の活用

#### `no_std`環境
- **制約**: 標準ライブラリが使えない
- **対応**:
  - `#![no_std]`の付与
  - `core`クレートのみ使用
  - `alloc`クレートを必要に応じて有効化
  - 文字列処理は`heapless`や`arrayvec`を検討

#### リアルタイム性
- **要求**: 割り込み応答、タイミング精度
- **対応**:
  - ビジーループの回避
  - タイマー/割り込みの活用
  - クリティカルセクションの最小化

#### ライフタイム管理
- **課題**: ESP32 HALではペリフェラルのライフタイムが複雑
- **対策**:
  - `'static`ライフタイムの活用
  - `unsafe`ブロックの適切な使用
  - `embassy`等の非同期フレームワーク検討

#### デバッグ環境
- **ツール**: 
  - `esp-println`でシリアル出力
  - `defmt`でログフレームワーク（ESP32対応確認必要）
  - JTAG/OpenOCDでのデバッグ
- **制約**: パニック時のバックトレースが限定的

### 非同期対応の検討
- **現状**: 同期API（ブロッキング）
- **ESP32での選択肢**:
  - `embassy-rs`: ESP32サポート進行中
  - `async-trait`: `no_std`で使用可能だが複雑化
  - FreeRTOSベース: `esp-idf-hal`経由
- **推奨**: まず同期APIで完成させ、後から非同期化を検討

### 状態管理とアーキテクチャ
- **現状**: `App`構造体内で状態保持
- **ESP32での考慮**:
  - グローバル状態は`static mut`または`Mutex`
  - イベント駆動: 割り込みハンドラとの連携
  - 状態機械: `enum`ベースの明示的な状態管理
- **将来**: Actor モデルや RTIC (Real-Time Interrupt-driven Concurrency) も検討可能

### ESP32実機検証で確認すべき項目チェックリスト

#### ビルド・書き込み
- [ ] `cargo build`の成功（PCクロスコンパイル）
- [ ] バイナリサイズの確認（Flash容量内か）
- [ ] `espflash`での書き込み成功
- [ ] シリアルモニタでの起動ログ確認

#### GPIO動作
- [ ] LED点滅が期待通りの周期か
- [ ] ピン番号の指定が正しいか
- [ ] プルアップ/プルダウン設定
- [ ] 入力ピンの読み取り

#### I2C通信
- [ ] I2Cデバイスのアドレススキャン
- [ ] センサからのデータ読み取り
- [ ] エラーハンドリング（NACKなど）
- [ ] クロック速度の適切性
- [ ] プルアップ抵抗の確認（ハードウェア）

#### パフォーマンス
- [ ] `tick()`の実行時間測定
- [ ] メモリ使用量（ヒープ/スタック）
- [ ] CPUクロック設定の影響
- [ ] Watchdogタイマーの扱い

#### エラーハンドリング
- [ ] パニック時の挙動（`esp-backtrace`）
- [ ] エラーからの回復
- [ ] ログ出力の適切性

#### 実環境要因
- [ ] 電源の安定性
- [ ] ノイズ耐性
- [ ] 温度変化の影響（必要に応じて）

---

**最終更新**: 2025-12-21  
**作成者**: GitHub Copilot (based on workspace analysis)
