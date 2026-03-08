//! # Platform PC Simulator Library
//!
//! PCシミュレータ用の補助ライブラリ。
//!
//! `main.rs` から利用するモックHALを公開し、examplesや統合テストでも
//! 同じ実装を再利用できるようにします。

pub mod climate_sim;
pub mod mock_hal;
