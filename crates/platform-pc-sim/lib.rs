//! # Platform PC Simulator Library
//!
//! PCシミュレータ用の補助ライブラリ。
//!
//! `main.rs` から利用するモックHALを公開し、examplesや統合テストでも
//! 同じ実装を再利用できるようにします。

pub mod bme280_mock;
pub mod climate_sim;
pub mod component_sim;
pub mod dashboard;
pub mod hc_sr04_mock;
pub mod lcd1602_mock;
pub mod mock_hal;
pub mod mpu6050_mock;
pub mod pwm_mock;
pub mod virtual_i2c;
pub mod web_dashboard;
pub mod wiring_config;
pub mod wiring_svg;
