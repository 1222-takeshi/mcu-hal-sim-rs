//! ESP32 GPIO アダプタ (generic adapter の type alias)

pub type Esp32OutputPin<P> = hal_api::adapter::GenericOutputPin<P>;
pub type Esp32InputPin<P> = hal_api::adapter::GenericInputPin<P>;
