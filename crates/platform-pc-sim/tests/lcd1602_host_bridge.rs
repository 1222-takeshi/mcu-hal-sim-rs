use embedded_hal::delay::DelayNs;
use hal_api::display::{TextDisplay16x2, TextFrame16x2};
use platform_esp32::lcd1602::{Lcd1602Display, LCD1602_ADDRESS_PRIMARY};
use platform_pc_sim::lcd1602_mock::MockLcd1602Device;
use platform_pc_sim::virtual_i2c::VirtualI2cBus;

struct NoopDelay;

impl DelayNs for NoopDelay {
    fn delay_ns(&mut self, _ns: u32) {}
}

#[test]
fn esp32_lcd1602_driver_writes_frame_to_host_mock() {
    let bus = VirtualI2cBus::new();
    let device = MockLcd1602Device::new();
    bus.attach_device(LCD1602_ADDRESS_PRIMARY, device.clone());

    let mut display = Lcd1602Display::new(bus.clone(), NoopDelay);

    let frame = TextFrame16x2::from_lines("Hello, World!   ", "Line 2 content  ");
    display.render(&frame).expect("render failed");

    let rendered = device.frame();
    assert_eq!(rendered.line(0), frame.line(0));
    assert_eq!(rendered.line(1), frame.line(1));
    assert!(device.write_count() > 0, "expected I2C writes after render");
    assert!(
        bus.attached_addresses().contains(&LCD1602_ADDRESS_PRIMARY),
        "device address not attached"
    );
}

#[test]
fn esp32_lcd1602_driver_renders_blank_frame() {
    let bus = VirtualI2cBus::new();
    let device = MockLcd1602Device::new();
    bus.attach_device(LCD1602_ADDRESS_PRIMARY, device.clone());

    let mut display = Lcd1602Display::new(bus, NoopDelay);

    let blank = TextFrame16x2::blank();
    display.render(&blank).expect("render blank failed");

    let rendered = device.frame();
    assert_eq!(rendered.line(0), blank.line(0));
    assert_eq!(rendered.line(1), blank.line(1));
}
