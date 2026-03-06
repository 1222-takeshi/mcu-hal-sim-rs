use core_app::App;
use platform_pc_sim::mock_hal::{MockI2c, MockPin};

#[test]
fn app_with_pc_sim_mocks_executes_expected_schedule() {
    let pin = MockPin::new(13);
    let i2c = MockI2c::new();
    let mut app = App::new(pin.clone(), i2c.clone());

    for _ in 0..499 {
        app.tick().unwrap();
    }

    assert_eq!(pin.history(), vec![true, false, true, false]);
    assert_eq!(i2c.read_count(), 0);

    app.tick().unwrap();

    assert_eq!(pin.history(), vec![true, false, true, false, true]);
    assert!(pin.level());
    assert_eq!(i2c.read_count(), 1);
    assert_eq!(i2c.last_read_addr(), Some(0x48));
    assert_eq!(i2c.last_read_len(), Some(4));
}

#[test]
fn app_with_pc_sim_mocks_survives_multiple_cycles() {
    let pin = MockPin::new(13);
    let i2c = MockI2c::new();
    let mut app = App::new(pin.clone(), i2c.clone());

    for _ in 0..1000 {
        app.tick().unwrap();
    }

    assert_eq!(pin.history().len(), 10);
    assert_eq!(i2c.read_count(), 2);
    assert_eq!(i2c.last_read_addr(), Some(0x48));
    assert_eq!(i2c.last_read_len(), Some(4));
}
