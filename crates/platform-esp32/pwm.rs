//! ESP32 PWM 出力アダプタ (generic adapter の type alias)
//!
//! `embedded-hal` v1.0 の `SetDutyCycle` を実装したピンを受け取り、
//! `hal_api::pwm::PwmOutput` に橋渡しします。実装とテストは
//! `hal-api::adapter::GenericPwmOutput` を参照してください。
//!
//! # 使用例（コンパイル確認用）
//!
//! ```ignore
//! // esp-hal の LEDC や McPWM ピンを Esp32PwmOutput でラップし、
//! // ServoDriver や L298nChannel と組み合わせる:
//! //
//! // let servo = ServoDriver::new(Esp32PwmOutput::new(ledc_channel));
//! // let motor_ch = L298nChannel::new(
//! //     Esp32OutputPin::new(gpio_in1),
//! //     Esp32OutputPin::new(gpio_in2),
//! //     Esp32PwmOutput::new(ledc_ena),
//! // );
//! ```

pub type Esp32PwmOutput<P> = hal_api::adapter::GenericPwmOutput<P>;
