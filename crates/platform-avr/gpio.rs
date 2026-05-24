//! AVR GPIO アダプタ (generic adapter の type alias)

pub type AvrOutputPin<P> = hal_api::adapter::GenericOutputPin<P>;
pub type AvrInputPin<P> = hal_api::adapter::GenericInputPin<P>;
