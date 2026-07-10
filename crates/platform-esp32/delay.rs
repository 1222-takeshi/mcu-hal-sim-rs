//! Delay adapter for ESP32: wraps embedded-hal DelayNs to provide platform delay.
//! (generic adapter の type alias — 実装とテストは `hal-api::adapter` を参照)

pub type Esp32Delay<D> = hal_api::adapter::GenericDelay<D>;
