#![no_std]

//! # Platform RP2040
//!
//! Raspberry Pi Pico (RP2040) 向けのプラットフォーム実装の足場です。
//!
//! `embedded-hal` v1.0 互換の GPIO / I2C 実装を `hal-api` に橋渡しする
//! 薄い adapter を提供します。
//!
//! これにより、Raspberry Pi Pico を追加するときも、
//! `core-app` 側を変えずに platform 層だけで吸収できます。

pub mod bme280;
pub mod gpio;
pub mod i2c;
pub mod lcd1602;
pub mod shared_i2c;
