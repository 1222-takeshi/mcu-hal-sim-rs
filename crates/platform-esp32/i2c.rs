//! ESP32 I2C アダプタ (generic adapter の type alias)

pub type Esp32I2c<I> = hal_api::adapter::GenericI2c<I>;
