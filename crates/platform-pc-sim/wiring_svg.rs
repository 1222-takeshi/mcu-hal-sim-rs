//! PCB-style animated wiring diagram SVG generator.
//!
//! Generates an inline SVG showing the board pins connected to attached
//! devices with colour-coded Bezier wires and CSS stroke-dashoffset
//! animations that simulate current flow on data lines.

use std::fmt::Write as _;

use crate::wiring_config::{ConnectionType, DeviceKind, WiringConfig};

/// Generate a self-contained SVG string for the given wiring config.
pub fn wiring_svg(config: &WiringConfig) -> String {
    let mut buf = String::with_capacity(12288);
    render(&mut buf, config);
    buf
}

// ── layout constants ────────────────────────────────────────────────────────
const W: i32 = 580;
const BASE_H: i32 = 520;

const BOARD_X: i32 = 16;
const BOARD_Y: i32 = 44;
const BOARD_W: i32 = 104;
const BOARD_H: i32 = 420;

// Board right edge (where pin dots sit)
const BOARD_R: i32 = BOARD_X + BOARD_W; // 120

// Pin Y positions on the board right edge
const P_SDA: i32 = BOARD_Y + 52;
const P_SCL: i32 = BOARD_Y + 96;
const P_VCC: i32 = BOARD_Y + 152;
const P_GND: i32 = BOARD_Y + 200;
/// PWM pin (Servo) — aligned with SRV label row.
const P_PWM: i32 = BOARD_Y + 263;
/// PWM pin (L298N motor driver enable) — aligned with MOT label row.
const P_MOT: i32 = BOARD_Y + 277;
/// GPIO pin (HC-SR04 trigger / camera boot).
const P_GPIO: i32 = BOARD_Y + 355;

// Devices
const DEV_X: i32 = 390;
const DEV_W: i32 = 160;
const DEV_H: i32 = 42;
const DEV_GAP: i32 = 10;

const VCC_RAIL_X: i32 = 214;
const GND_RAIL_X: i32 = 240;
const SDA_RAIL_X: i32 = 280;
const SCL_RAIL_X: i32 = 306;

#[allow(clippy::write_with_newline)]
fn render(out: &mut String, config: &WiringConfig) {
    let device_rows = config.devices.len().max(1) as i32;
    let total_h = device_rows * (DEV_H + DEV_GAP) - DEV_GAP;
    let content_h = total_h.max(BOARD_H);
    let svg_h = (BOARD_Y + content_h + 56).max(BASE_H);
    let dev_start_y = BOARD_Y + (content_h - total_h) / 2;
    let layouts: Vec<_> = config
        .devices
        .iter()
        .enumerate()
        .map(|(i, dev)| {
            let dy = dev_start_y + i as i32 * (DEV_H + DEV_GAP);
            let mid_y = dy + DEV_H / 2;
            (dev, dy, mid_y)
        })
        .collect();

    // SVG open + styles
    let _ = write!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {svg_h}" style="width:100%;height:auto;background:#0c1a12;border-radius:8px;display:block">
<defs><style>
.pcb-board{{fill:#152e15;stroke:#3d7a3d;stroke-width:2}}
.pcb-lbl{{fill:#7bc47b;font:bold 11px monospace}}
.pcb-sub{{fill:#5a9a5a;font:9px monospace}}
.pcb-pin{{fill:#9ab;font:10px monospace}}
.dev-box{{fill:#1a2333;stroke:#3a5888;stroke-width:1.5}}
.dev-lbl{{fill:#7ab0e8;font:bold 10px monospace}}
.dev-sub{{fill:#6888aa;font:9px monospace}}
.dev-pin{{fill:#8899bb;font:8px monospace}}
.profile-lbl{{fill:#a0c8a0;font:bold 10px monospace;opacity:0.85}}
.w-sda{{fill:none;stroke:#4488ff;stroke-width:2.2;stroke-dasharray:8 4;animation:wiring-flow 1.2s linear infinite}}
.w-scl{{fill:none;stroke:#ffdd44;stroke-width:1.8;stroke-dasharray:8 4;animation:wiring-flow 1.7s linear infinite reverse}}
.w-vcc{{fill:none;stroke:#e55;stroke-width:1.6}}
.w-gnd{{fill:none;stroke:#556;stroke-width:1.6}}
.w-gpio{{fill:none;stroke:#ff9944;stroke-width:1.8;stroke-dasharray:6 3;animation:wiring-flow 2s linear infinite}}
.w-pwm{{fill:none;stroke:#bb88ff;stroke-width:1.8;stroke-dasharray:6 3;animation:wiring-flow 1.5s linear infinite}}
.w-bus-feed{{opacity:0.9}}
.w-bus-trunk{{opacity:0.95}}
.w-bus-branch{{opacity:0.92}}
@keyframes wiring-flow{{from{{stroke-dashoffset:24}}to{{stroke-dashoffset:0}}}}
.dot-sda{{fill:#4488ff}}.dot-scl{{fill:#ffdd44}}.dot-vcc{{fill:#e55}}.dot-gnd{{fill:#556}}.dot-gpio{{fill:#ff9944}}.dot-pwm{{fill:#bb88ff}}
.leg{{fill:#888;font:9px monospace}}
</style></defs>
"#
    );

    // Board PCB
    let board_label = config.board.name();
    let mcu_label = config.board.mcu();
    let cx = BOARD_X + BOARD_W / 2;
    let _ = write!(
        out,
        r#"<rect x="{BOARD_X}" y="{BOARD_Y}" width="{BOARD_W}" height="{BOARD_H}" rx="6" class="pcb-board"/>
<text x="{cx}" y="{}" class="pcb-lbl" text-anchor="middle">{board_label}</text>
<text x="{cx}" y="{}" class="pcb-sub" text-anchor="middle">{mcu_label}</text>
"#,
        BOARD_Y + 20,
        BOARD_Y + 33,
    );

    // SensorProfile label (top-right corner)
    let profile_name = config.sensor_profile.display_name();
    let _ = write!(
        out,
        r#"<text x="{}" y="28" class="profile-lbl" text-anchor="end">Profile: {profile_name}</text>
"#,
        W - 8,
    );

    // Board pin dots + labels
    for (cy, dot_cls, label) in [
        (P_SDA, "dot-sda", format!("SDA/{}", config.sda_pin)),
        (P_SCL, "dot-scl", format!("SCL/{}", config.scl_pin)),
        (P_VCC, "dot-vcc", config.power_pin.clone()),
        (P_GND, "dot-gnd", config.ground_pin.clone()),
    ] {
        let _ = write!(
            out,
            r#"<circle cx="{BOARD_R}" cy="{cy}" r="4" class="{dot_cls}"/>
<text x="{}" y="{}" class="pcb-pin" text-anchor="end">{label}</text>
"#,
            BOARD_R - 7,
            cy + 4,
        );
    }
    let has_servo = config
        .devices
        .iter()
        .any(|device| device.kind == DeviceKind::Servo);
    let has_motor = config
        .devices
        .iter()
        .any(|device| device.kind == DeviceKind::L298n);
    let has_sonar = config
        .devices
        .iter()
        .any(|device| device.kind == DeviceKind::HcSr04);
    let has_camera = config
        .devices
        .iter()
        .any(|device| device.kind == DeviceKind::Esp32Cam);

    for (enabled, cy, dot_cls) in [
        (has_servo, P_PWM, "dot-pwm"),
        (has_motor, P_MOT, "dot-pwm"),
        (has_sonar || has_camera, P_GPIO, "dot-gpio"),
    ] {
        if !enabled {
            continue;
        }
        let _ = write!(
            out,
            r#"<circle cx="{BOARD_R}" cy="{cy}" r="4" class="{dot_cls}"/>
"#
        );
    }

    // GPIO pin group label block
    if has_sonar || has_camera {
        let gpio_y = BOARD_Y + 330;
        let _ = write!(
            out,
            r#"<text x="{}" y="{}" class="pcb-sub" text-anchor="middle">GPIO</text>
"#,
            cx,
            gpio_y - 6,
        );
        let mut gpio_line_index = 0;
        if has_sonar {
            let trig_y = gpio_y + 8 + gpio_line_index * 14;
            let echo_y = trig_y + 14;
            let _ = write!(
                out,
                r#"<text x="{}" y="{}" class="pcb-pin" text-anchor="end">TRIG/{}</text>
<text x="{}" y="{}" class="pcb-pin" text-anchor="end">ECHO/{}</text>
"#,
                BOARD_R - 7,
                trig_y,
                config.trig_pin,
                BOARD_R - 7,
                echo_y,
                config.echo_pin,
            );
            gpio_line_index += 2;
        }
        if has_camera {
            let cam_y = gpio_y + 8 + gpio_line_index * 14;
            let _ = write!(
                out,
                r#"<text x="{}" y="{}" class="pcb-pin" text-anchor="end">CAM/{}</text>
"#,
                BOARD_R - 7,
                cam_y,
                config.cam_pin,
            );
        }
    }

    // PWM pin group label block
    if has_servo || has_motor {
        let pwm_y = BOARD_Y + 255;
        let _ = write!(
            out,
            r#"<text x="{}" y="{}" class="pcb-sub" text-anchor="middle">PWM</text>
"#,
            cx,
            pwm_y - 6,
        );
        if has_servo {
            let _ = write!(
                out,
                r#"<text x="{}" y="{}" class="pcb-pin" text-anchor="end">SRV/{}</text>
"#,
                BOARD_R - 7,
                pwm_y + 8,
                config.servo_pin,
            );
        }
        if has_motor {
            let mot_y = if has_servo { pwm_y + 22 } else { pwm_y + 8 };
            let _ = write!(
                out,
                r#"<text x="{}" y="{}" class="pcb-pin" text-anchor="end">MOT/{}</text>
"#,
                BOARD_R - 7,
                mot_y,
                config.motor_pin,
            );
        }
    }

    let draw_bus_feed = |out: &mut String, cls: &str, board_y: i32, rail_x: i32, dot_cls: &str| {
        let cp = BOARD_R + (rail_x - BOARD_R) * 6 / 10;
        let _ = write!(
            out,
            r#"<path class="{cls} w-bus-feed" d="M {BOARD_R} {board_y} C {cp} {board_y} {cp} {board_y} {rail_x} {board_y}"/>
<circle cx="{rail_x}" cy="{board_y}" r="3" class="{dot_cls}"/>
"#
        );
    };
    let draw_bus_trunk = |out: &mut String, cls: &str, rail_x: i32, top_y: i32, bottom_y: i32| {
        let _ = write!(
            out,
            r#"<path class="{cls} w-bus-trunk" d="M {rail_x} {top_y} L {rail_x} {bottom_y}"/>
"#
        );
    };
    let trunk_span =
        |feed_y: i32, top_y: i32, bottom_y: i32| (top_y.min(feed_y), bottom_y.max(feed_y));

    let vcc_top = layouts.iter().map(|(_, dy, _)| dy + 8).min();
    let vcc_bottom = layouts.iter().map(|(_, dy, _)| dy + 8).max();
    if let (Some(top_y), Some(bottom_y)) = (vcc_top, vcc_bottom) {
        draw_bus_feed(out, "w-vcc", P_VCC, VCC_RAIL_X, "dot-vcc");
        let (vcc_top, vcc_bottom) = trunk_span(P_VCC, top_y, bottom_y);
        draw_bus_trunk(out, "w-vcc", VCC_RAIL_X, vcc_top, vcc_bottom);
    }

    let gnd_top = layouts.iter().map(|(_, dy, _)| dy + DEV_H - 8).min();
    let gnd_bottom = layouts.iter().map(|(_, dy, _)| dy + DEV_H - 8).max();
    if let (Some(top_y), Some(bottom_y)) = (gnd_top, gnd_bottom) {
        draw_bus_feed(out, "w-gnd", P_GND, GND_RAIL_X, "dot-gnd");
        let (gnd_top, gnd_bottom) = trunk_span(P_GND, top_y, bottom_y);
        draw_bus_trunk(out, "w-gnd", GND_RAIL_X, gnd_top, gnd_bottom);
    }

    let i2c_top = layouts
        .iter()
        .filter_map(|(dev, _, mid_y)| {
            (dev.kind.connection_type() == ConnectionType::I2c).then_some(mid_y - 6)
        })
        .min();
    let i2c_bottom = layouts
        .iter()
        .filter_map(|(dev, _, mid_y)| {
            (dev.kind.connection_type() == ConnectionType::I2c).then_some(mid_y + 4)
        })
        .max();
    if let (Some(top_y), Some(bottom_y)) = (i2c_top, i2c_bottom) {
        draw_bus_feed(out, "w-sda", P_SDA, SDA_RAIL_X, "dot-sda");
        let (sda_top, sda_bottom) = trunk_span(P_SDA, top_y, bottom_y);
        draw_bus_trunk(out, "w-sda", SDA_RAIL_X, sda_top, sda_bottom);
        draw_bus_feed(out, "w-scl", P_SCL, SCL_RAIL_X, "dot-scl");
        let (scl_top, scl_bottom) = trunk_span(P_SCL, top_y, bottom_y);
        draw_bus_trunk(out, "w-scl", SCL_RAIL_X, scl_top, scl_bottom);
    }

    for (dev, dy, mid_y) in layouts {
        let conn = dev.kind.connection_type();
        let cp = BOARD_R + (DEV_X - BOARD_R) * 6 / 10;
        let bus_label_x = DEV_X - 8;

        // Device box
        let _ = write!(
            out,
            r#"<rect x="{DEV_X}" y="{dy}" width="{DEV_W}" height="{DEV_H}" rx="4" class="dev-box"/>
<text x="{}" y="{}" class="dev-lbl">{}</text>
"#,
            DEV_X + 8,
            dy + 15,
            dev.kind.label(),
        );
        if let Some(addr) = dev.address {
            let _ = write!(
                out,
                r#"<text x="{}" y="{}" class="dev-sub">I²C addr: 0x{addr:02X}</text>
"#,
                DEV_X + 8,
                dy + 28,
            );
        }

        // VCC wire + entry dot
        let y_vcc = dy + 8;
        let _ = write!(
            out,
            r#"<path class="w-vcc w-bus-branch" d="M {VCC_RAIL_X} {y_vcc} L {DEV_X} {y_vcc}"/>
<circle cx="{DEV_X}" cy="{y_vcc}" r="3" class="dot-vcc"/>
"#,
        );
        if config.show_bus_labels {
            let _ = write!(
                out,
                r#"<text x="{}" y="{}" text-anchor="end" class="dev-pin">VCC</text>
"#,
                bus_label_x,
                y_vcc - 2,
            );
        }

        // GND wire + entry dot
        let y_gnd = dy + DEV_H - 8;
        let _ = write!(
            out,
            r#"<path class="w-gnd w-bus-branch" d="M {GND_RAIL_X} {y_gnd} L {DEV_X} {y_gnd}"/>
<circle cx="{DEV_X}" cy="{y_gnd}" r="3" class="dot-gnd"/>
"#,
        );
        if config.show_bus_labels {
            let _ = write!(
                out,
                r#"<text x="{}" y="{}" text-anchor="end" class="dev-pin">GND</text>
"#,
                bus_label_x,
                y_gnd + 8,
            );
        }

        match conn {
            ConnectionType::I2c => {
                let y_sda = mid_y - 6;
                let y_scl = mid_y + 4;
                let _ = write!(
                    out,
                    r#"<path class="w-sda w-bus-branch" d="M {SDA_RAIL_X} {y_sda} L {DEV_X} {y_sda}"/>
<circle cx="{DEV_X}" cy="{y_sda}" r="3" class="dot-sda"/>
<path class="w-scl w-bus-branch" d="M {SCL_RAIL_X} {y_scl} L {DEV_X} {y_scl}"/>
<circle cx="{DEV_X}" cy="{y_scl}" r="3" class="dot-scl"/>
"#,
                );
                if config.show_bus_labels {
                    let _ = write!(
                        out,
                        r#"<text x="{}" y="{}" text-anchor="end" class="dev-pin">SDA</text>
<text x="{}" y="{}" text-anchor="end" class="dev-pin">SCL</text>
"#,
                        bus_label_x,
                        y_sda - 2,
                        bus_label_x,
                        y_scl - 2,
                    );
                }
            }
            ConnectionType::Gpio => {
                let trig_pin = &config.trig_pin;
                let echo_pin = &config.echo_pin;
                let cam_pin = &config.cam_pin;
                let is_camera = dev.kind == crate::wiring_config::DeviceKind::Esp32Cam;
                let pin_label = if is_camera {
                    format!("GPIO:{cam_pin}")
                } else {
                    format!("TRIG:{trig_pin} / ECHO:{echo_pin}")
                };
                let _ = write!(
                    out,
                    r#"<path class="w-gpio" d="M {BOARD_R} {P_GPIO} C {cp} {P_GPIO} {cp} {mid_y} {DEV_X} {mid_y}"/>
<circle cx="{DEV_X}" cy="{mid_y}" r="3" class="dot-gpio"/>
<text x="{}" y="{}" class="dev-sub">{pin_label}</text>
"#,
                    DEV_X + 8,
                    dy + DEV_H - 4,
                );
            }
            ConnectionType::Pwm => {
                let (origin_y, pwm_pin) = if dev.kind == DeviceKind::L298n {
                    (P_MOT, &config.motor_pin)
                } else {
                    (P_PWM, &config.servo_pin)
                };
                let _ = write!(
                    out,
                    r#"<path class="w-pwm" d="M {BOARD_R} {origin_y} C {cp} {origin_y} {cp} {mid_y} {DEV_X} {mid_y}"/>
<circle cx="{DEV_X}" cy="{mid_y}" r="3" class="dot-pwm"/>
<text x="{}" y="{}" class="dev-sub">PWM:{pwm_pin}</text>
"#,
                    DEV_X + 8,
                    dy + DEV_H - 4,
                );
            }
        }
    }

    // Legend at bottom
    let leg_y = svg_h - 10;
    let _ = write!(
        out,
        r#"<text x="{}" y="{leg_y}" class="leg" text-anchor="middle"><tspan class="dot-sda">━━</tspan> SDA <tspan dx="6" class="dot-scl">━━</tspan> SCL <tspan dx="6" class="dot-vcc">━━</tspan> VCC <tspan dx="6" class="dot-gnd">━━</tspan> GND <tspan dx="6" class="dot-gpio">━━</tspan> GPIO <tspan dx="6" class="dot-pwm">━━</tspan> PWM</text>
"#,
        W / 2,
    );

    out.push_str("</svg>");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dashboard::BoardProfile;
    use crate::wiring_config::WiringConfig;

    #[test]
    fn wiring_svg_contains_board_label() {
        let cfg = WiringConfig::from_board(BoardProfile::OriginalEsp32);
        let svg = wiring_svg(&cfg);
        assert!(svg.contains("original ESP32"), "missing board label");
        assert!(svg.contains("ESP32"), "missing MCU label");
    }

    #[test]
    fn wiring_svg_contains_device_labels() {
        let cfg = WiringConfig::from_board(BoardProfile::OriginalEsp32);
        let svg = wiring_svg(&cfg);
        assert!(svg.contains("BME280"));
        assert!(svg.contains("MPU6050"));
        assert!(svg.contains("LCD1602"));
        assert!(svg.contains("HC-SR04"));
        assert!(svg.contains("BH1750"));
        assert!(svg.contains("Servo"));
        assert!(svg.contains("L298N"));
        assert!(svg.contains("ESP32-CAM"));
    }

    #[test]
    fn wiring_svg_contains_pin_labels() {
        let cfg = WiringConfig::from_board(BoardProfile::OriginalEsp32);
        let svg = wiring_svg(&cfg);
        assert!(svg.contains("GPIO21"), "missing SDA pin");
        assert!(svg.contains("GPIO22"), "missing SCL pin");
        assert!(svg.contains("3V3"), "missing power pin");
        assert!(svg.contains("GPIO13"), "missing servo PWM pin");
        assert!(svg.contains("GPIO25"), "missing motor ENA pin");
        assert!(svg.contains("GPIO0"), "missing cam pin");
    }

    #[test]
    fn wiring_svg_contains_animation_css() {
        let cfg = WiringConfig::from_board(BoardProfile::OriginalEsp32);
        let svg = wiring_svg(&cfg);
        assert!(svg.contains("wiring-flow"), "missing CSS animation name");
        assert!(svg.contains("stroke-dasharray"), "missing dash array");
        assert!(svg.contains("w-sda"), "missing SDA wire class");
        assert!(svg.contains("w-scl"), "missing SCL wire class");
        assert!(svg.contains("w-pwm"), "missing PWM wire class");
    }

    #[test]
    fn wiring_svg_is_valid_svg_element() {
        let cfg = WiringConfig::from_board(BoardProfile::OriginalEsp32);
        let svg = wiring_svg(&cfg);
        assert!(svg.starts_with("<svg "));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn wiring_svg_contains_profile_label() {
        use crate::wiring_config::SensorProfile;
        let cfg =
            WiringConfig::from_board_with_sensors(BoardProfile::OriginalEsp32, SensorProfile::Full);
        let svg = wiring_svg(&cfg);
        assert!(svg.contains("Profile:"), "missing profile indicator");
        assert!(svg.contains("Full"), "missing profile name");
    }

    #[test]
    fn wiring_svg_profile_label_changes_with_profile() {
        use crate::wiring_config::SensorProfile;
        let full =
            WiringConfig::from_board_with_sensors(BoardProfile::OriginalEsp32, SensorProfile::Full);
        let climate = WiringConfig::from_board_with_sensors(
            BoardProfile::OriginalEsp32,
            SensorProfile::ClimateStation,
        );
        let svg_full = wiring_svg(&full);
        let svg_climate = wiring_svg(&climate);
        assert!(svg_full.contains("Full"));
        assert!(svg_climate.contains("Climate"));
    }

    #[test]
    fn wiring_svg_omits_redundant_device_pin_labels() {
        let cfg = WiringConfig::from_board(BoardProfile::OriginalEsp32);
        let svg = wiring_svg(&cfg);
        assert!(
            !svg.contains(r#"class="dev-pin""#),
            "shared-bus layouts should not repeat per-device pin text labels"
        );
    }

    #[test]
    fn wiring_svg_omits_redundant_device_pin_labels_in_compact_profile() {
        use crate::wiring_config::SensorProfile;

        let cfg = WiringConfig::from_board_with_sensors(
            BoardProfile::OriginalEsp32,
            SensorProfile::Minimal,
        );
        let svg = wiring_svg(&cfg);
        assert!(
            !svg.contains(r#"class="dev-pin""#),
            "compact shared-bus layouts should not reintroduce per-device pin text labels"
        );
        assert_eq!(svg.matches(r#"class="w-vcc w-bus-trunk""#).count(), 1);
        assert_eq!(svg.matches(r#"class="w-gnd w-bus-trunk""#).count(), 1);
        assert_eq!(svg.matches(r#"class="w-sda w-bus-trunk""#).count(), 1);
        assert_eq!(svg.matches(r#"class="w-scl w-bus-trunk""#).count(), 1);
        assert_eq!(svg.matches(r#"class="w-vcc w-bus-branch""#).count(), 2);
        assert_eq!(svg.matches(r#"class="w-gnd w-bus-branch""#).count(), 2);
        assert_eq!(svg.matches(r#"class="w-sda w-bus-branch""#).count(), 2);
        assert_eq!(svg.matches(r#"class="w-scl w-bus-branch""#).count(), 2);
    }

    #[test]
    fn wiring_svg_renders_device_pin_labels_when_enabled() {
        let cfg = WiringConfig::from_board(BoardProfile::OriginalEsp32).with_bus_labels(true);
        let svg = wiring_svg(&cfg);
        assert!(svg.contains(r#"class="dev-pin""#));
        assert!(svg.contains(">VCC<"));
        assert!(svg.contains(">GND<"));
        assert!(svg.contains(">SDA<"));
        assert!(svg.contains(">SCL<"));
    }

    #[test]
    fn wiring_svg_places_bus_labels_left_of_device_boxes_when_enabled() {
        let cfg = WiringConfig::from_board(BoardProfile::OriginalEsp32).with_bus_labels(true);
        let svg = wiring_svg(&cfg);
        assert!(
            svg.contains(r#"text-anchor="end" class="dev-pin""#),
            "device pin labels should anchor from the left-side bus margin"
        );
        assert!(
            !svg.contains(&format!(r#"<text x="{}" y=""#, DEV_X + 6)),
            "device pin labels should not be rendered inside the device box"
        );
    }

    #[test]
    fn wiring_svg_contains_vcc_gnd_dots() {
        let cfg = WiringConfig::from_board(BoardProfile::OriginalEsp32);
        let svg = wiring_svg(&cfg);
        assert!(svg.contains("dot-vcc"), "missing VCC dots");
        assert!(svg.contains("dot-gnd"), "missing GND dots");
    }

    #[test]
    fn wiring_svg_uses_shared_bus_trunks_for_dense_layout() {
        let cfg = WiringConfig::from_board(BoardProfile::OriginalEsp32);
        let svg = wiring_svg(&cfg);
        assert_eq!(svg.matches(r#"class="w-vcc w-bus-trunk""#).count(), 1);
        assert_eq!(svg.matches(r#"class="w-gnd w-bus-trunk""#).count(), 1);
        assert_eq!(svg.matches(r#"class="w-sda w-bus-trunk""#).count(), 1);
        assert_eq!(svg.matches(r#"class="w-scl w-bus-trunk""#).count(), 1);
        assert_eq!(svg.matches(r#"class="w-vcc w-bus-branch""#).count(), 11);
        assert_eq!(svg.matches(r#"class="w-sda w-bus-branch""#).count(), 7);
        assert_eq!(svg.matches(r#"class="w-scl w-bus-branch""#).count(), 7);
    }

    #[test]
    fn wiring_svg_keeps_pwm_and_gpio_pin_labels_single_sourced() {
        let cfg = WiringConfig::from_board(BoardProfile::OriginalEsp32);
        let svg = wiring_svg(&cfg);
        assert_eq!(svg.matches("SRV/GPIO13").count(), 1);
        assert_eq!(svg.matches("MOT/GPIO25").count(), 1);
        assert!(
            !svg.contains("GPIO/GPIO23"),
            "generic GPIO label should not duplicate the dedicated GPIO block"
        );
    }

    #[test]
    fn wiring_svg_expands_height_for_large_device_sets() {
        let cfg = WiringConfig::from_board(BoardProfile::OriginalEsp32);
        let svg = wiring_svg(&cfg);
        assert!(
            svg.contains(r#"viewBox="0 0 580 662""#),
            "full wiring SVG should expand vertically for dense layouts"
        );
    }

    #[test]
    fn wiring_svg_keeps_base_height_for_small_device_sets() {
        use crate::wiring_config::SensorProfile;

        let cfg = WiringConfig::from_board_with_sensors(
            BoardProfile::OriginalEsp32,
            SensorProfile::Minimal,
        );
        let svg = wiring_svg(&cfg);
        assert!(
            svg.contains(r#"viewBox="0 0 580 520""#),
            "minimal wiring SVG should keep the compact default height"
        );
    }

    #[test]
    fn wiring_svg_hides_unused_pwm_and_gpio_groups_in_minimal_profile() {
        use crate::wiring_config::SensorProfile;

        let cfg = WiringConfig::from_board_with_sensors(
            BoardProfile::OriginalEsp32,
            SensorProfile::Minimal,
        );
        let svg = wiring_svg(&cfg);

        assert!(!svg.contains(">PWM<"));
        assert!(!svg.contains(">GPIO<"));
        assert!(!svg.contains("SRV/GPIO13"));
        assert!(!svg.contains("MOT/GPIO25"));
        assert!(!svg.contains("TRIG/GPIO5"));
        assert!(!svg.contains("CAM/GPIO0"));
    }

    #[test]
    fn wiring_svg_hides_camera_board_label_when_camera_is_not_selected() {
        use crate::wiring_config::SensorProfile;

        let cfg = WiringConfig::from_board_with_sensors(
            BoardProfile::OriginalEsp32,
            SensorProfile::RobotBase,
        );
        let svg = wiring_svg(&cfg);

        assert!(svg.contains("TRIG/GPIO5"));
        assert!(svg.contains("ECHO/GPIO18"));
        assert!(!svg.contains("CAM/GPIO0"));
    }

    #[test]
    fn wiring_svg_hides_nano_camera_placeholder_when_unsupported() {
        let cfg = WiringConfig::from_board(BoardProfile::ArduinoNano);
        let svg = wiring_svg(&cfg);

        assert!(!svg.contains("CAM/N/A"));
        assert!(!svg.contains("GPIO:N/A"));
    }

    #[test]
    fn wiring_svg_connects_sparse_minimal_bus_trunks_back_to_board_feed() {
        use crate::wiring_config::SensorProfile;

        let cfg = WiringConfig::from_board_with_sensors(
            BoardProfile::OriginalEsp32,
            SensorProfile::Minimal,
        );
        let svg = wiring_svg(&cfg);

        assert!(svg.contains(r#"<path class="w-vcc w-bus-trunk" d="M 214 196 L 214 267"/>"#));
        assert!(svg.contains(r#"<path class="w-sda w-bus-trunk" d="M 280 96 L 280 284"/>"#));
        assert!(svg.contains(r#"<path class="w-scl w-bus-trunk" d="M 306 140 L 306 284"/>"#));
    }

    #[test]
    fn wiring_svg_limits_sparse_power_trunks_to_their_own_branch_ranges() {
        use crate::wiring_config::SensorProfile;

        let cfg = WiringConfig::from_board_with_sensors(
            BoardProfile::OriginalEsp32,
            SensorProfile::Minimal,
        );
        let svg = wiring_svg(&cfg);

        assert!(svg.contains(r#"<path class="w-vcc w-bus-trunk" d="M 214 196 L 214 267"/>"#));
        assert!(svg.contains(r#"<path class="w-gnd w-bus-trunk" d="M 240 241 L 240 293"/>"#));
        assert!(!svg.contains(r#"<path class="w-vcc w-bus-trunk" d="M 214 196 L 214 293"/>"#));
        assert!(!svg.contains(r#"<path class="w-gnd w-bus-trunk" d="M 240 215 L 240 293"/>"#));
    }

    #[test]
    fn wiring_svg_connects_robot_i2c_trunks_back_to_board_feed() {
        use crate::wiring_config::SensorProfile;

        let cfg = WiringConfig::from_board_with_sensors(
            BoardProfile::OriginalEsp32,
            SensorProfile::RobotBase,
        );
        let svg = wiring_svg(&cfg);

        assert!(svg.contains(r#"<path class="w-sda w-bus-trunk" d="M 280 96 L 280 206"/>"#));
        assert!(svg.contains(r#"<path class="w-scl w-bus-trunk" d="M 306 140 L 306 206"/>"#));
    }

    #[test]
    fn wiring_svg_limits_robot_power_trunks_to_their_own_branch_ranges() {
        use crate::wiring_config::SensorProfile;

        let cfg = WiringConfig::from_board_with_sensors(
            BoardProfile::OriginalEsp32,
            SensorProfile::RobotBase,
        );
        let svg = wiring_svg(&cfg);

        assert!(svg.contains(r#"<path class="w-vcc w-bus-trunk" d="M 214 137 L 214 345"/>"#));
        assert!(svg.contains(r#"<path class="w-gnd w-bus-trunk" d="M 240 163 L 240 371"/>"#));
        assert!(!svg.contains(r#"<path class="w-vcc w-bus-trunk" d="M 214 137 L 214 371"/>"#));
        assert!(!svg.contains(r#"<path class="w-gnd w-bus-trunk" d="M 240 137 L 240 371"/>"#));
    }
}
