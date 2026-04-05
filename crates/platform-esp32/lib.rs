#![no_std]

//! # Platform ESP32
//!
//! original ESP32 向けプラットフォーム実装の足場です。
//!
//! 現段階では `esp-hal` そのものへの直接依存はまだ持たず、
//! `embedded-hal` v1.0 互換の GPIO / I2C 実装を `hal-api` に
//! 接続するラッパーを提供します。`BME280` / `LCD1602` の
//! board 非依存 driver 本体は `reference-drivers` crate にあり、
//! この crate では original ESP32 向けの参照経路として re-export します。

pub mod bme280;
pub mod delay;
pub mod gpio;
pub mod hc_sr04;
pub mod i2c;
pub mod lcd1602;
pub mod mpu6050;
pub mod shared_i2c;

pub use delay::Esp32Delay;
