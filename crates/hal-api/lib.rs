#![no_std]

//! # HAL API
//!
//! マイコン向けハードウェア抽象化層（HAL）のtrait定義。
//!
//! このクレートは、GPIO、I2Cなどの周辺機器に対する統一されたインターフェースを提供します。
//! プラットフォーム固有の実装は、これらのtraitを実装することでアプリケーションと互換性を持ちます。
//!
//! # Examples
//!
//! ```
//! use hal_api::gpio::OutputPin;
//! use hal_api::i2c::I2cBus;
//! use hal_api::error::{GpioError, I2cError};
//!
//! // GPIO出力ピンの実装例
//! struct MyPin;
//!
//! impl OutputPin for MyPin {
//!     type Error = GpioError;
//!
//!     fn set_high(&mut self) -> Result<(), Self::Error> {
//!         // プラットフォーム固有の実装
//!         Ok(())
//!     }
//!
//!     fn set_low(&mut self) -> Result<(), Self::Error> {
//!         // プラットフォーム固有の実装
//!         Ok(())
//!     }
//! }
//! ```

pub mod actuator;
pub mod display;
pub mod distance;
pub mod error;
pub mod gpio;
pub mod i2c;
pub mod imu;
pub mod sensor;
