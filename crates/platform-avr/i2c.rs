//! AVR I2C アダプタ (generic adapter の type alias)

pub type AvrI2c<I> = hal_api::adapter::GenericI2c<I>;
