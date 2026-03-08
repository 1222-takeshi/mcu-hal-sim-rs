#![no_std]

//! # Platform ESP32
//!
//! original ESP32 向けプラットフォーム実装の足場です。
//!
//! 現段階では `esp-hal` そのものへの直接依存はまだ持たず、
//! `embedded-hal` v1.0 互換の GPIO / I2C 実装を `hal-api` に
//! 接続するラッパーを提供します。将来の実機対応では
//! `esp-hal` の型をこれらのアダプタへ流し込む想定です。

pub mod bme280;
pub mod gpio;
pub mod i2c;
pub mod lcd1602;
pub mod shared_i2c;
