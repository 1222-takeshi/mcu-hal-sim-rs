//! RP2040 GPIO アダプタ (generic adapter の type alias)

pub type Rp2040OutputPin<P> = hal_api::adapter::GenericOutputPin<P>;
pub type Rp2040InputPin<P> = hal_api::adapter::GenericInputPin<P>;
