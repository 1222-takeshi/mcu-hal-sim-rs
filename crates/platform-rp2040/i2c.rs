//! RP2040 I2C アダプタ (generic adapter の type alias)

pub type Rp2040I2c<I> = hal_api::adapter::GenericI2c<I>;
