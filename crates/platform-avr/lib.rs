#![no_std]

//! # Platform AVR
//!
//! AVR 系 board 向けのプラットフォーム実装の足場です。
//!
//! 現段階では `arduino-hal` や `avr-hal` の具体型へ直接結合せず、
//! `embedded-hal` v1.0 互換の GPIO / I2C 実装を `hal-api` に橋渡しする
//! 薄い adapter を提供します。
//!
//! これにより、classic Arduino Nano のような AVR board を追加するときも、
//! `core-app` 側を変えずに platform 層だけで吸収できます。

pub mod gpio;
pub mod i2c;
