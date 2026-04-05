//! PCB-style animated wiring diagram SVG generator.
//!
//! Generates an inline SVG showing the board pins connected to attached
//! devices with colour-coded Bezier wires and CSS stroke-dashoffset
//! animations that simulate current flow on data lines.

use std::fmt::Write as _;

use crate::wiring_config::{ConnectionType, WiringConfig};

/// Generate a self-contained SVG string for the given wiring config.
pub fn wiring_svg(config: &WiringConfig) -> String {
    let mut buf = String::with_capacity(8192);
    render(&mut buf, config);
    buf
}

// ── layout constants ────────────────────────────────────────────────────────
const W: i32 = 560;
const H: i32 = 380;

const BOARD_X: i32 = 16;
const BOARD_Y: i32 = 44;
const BOARD_W: i32 = 104;
const BOARD_H: i32 = 292;

// Board right edge (where pin dots sit)
const BOARD_R: i32 = BOARD_X + BOARD_W; // 120

// Pin Y positions on the board right edge
const P_SDA: i32 = BOARD_Y + 52;
const P_SCL: i32 = BOARD_Y + 96;
const P_VCC: i32 = BOARD_Y + 152;
const P_GND: i32 = BOARD_Y + 200;

// Devices
const DEV_X: i32 = 382;
const DEV_W: i32 = 148;
const DEV_H: i32 = 52;
const DEV_GAP: i32 = 14;

#[allow(clippy::write_with_newline)]
fn render(out: &mut String, config: &WiringConfig) {
    // SVG open + styles
    let _ = write!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" style="width:100%;max-height:{H}px;background:#0c1a12;border-radius:8px;display:block">
<defs><style>
.pcb-board{{fill:#152e15;stroke:#3d7a3d;stroke-width:2}}
.pcb-lbl{{fill:#7bc47b;font:bold 11px monospace}}
.pcb-sub{{fill:#5a9a5a;font:9px monospace}}
.pcb-pin{{fill:#9ab;font:10px monospace}}
.dev-box{{fill:#1a2333;stroke:#3a5888;stroke-width:1.5}}
.dev-lbl{{fill:#7ab0e8;font:bold 10px monospace}}
.dev-sub{{fill:#6888aa;font:9px monospace}}
.w-sda{{fill:none;stroke:#4488ff;stroke-width:2.2;stroke-dasharray:8 4;animation:wiring-flow 1.2s linear infinite}}
.w-scl{{fill:none;stroke:#ffdd44;stroke-width:1.8;stroke-dasharray:8 4;animation:wiring-flow 1.7s linear infinite reverse}}
.w-vcc{{fill:none;stroke:#e55;stroke-width:1.6}}
.w-gnd{{fill:none;stroke:#556;stroke-width:1.6}}
.w-gpio{{fill:none;stroke:#ff9944;stroke-width:1.8;stroke-dasharray:6 3;animation:wiring-flow 2s linear infinite}}
@keyframes wiring-flow{{from{{stroke-dashoffset:24}}to{{stroke-dashoffset:0}}}}
.dot-sda{{fill:#4488ff}}.dot-scl{{fill:#ffdd44}}.dot-vcc{{fill:#e55}}.dot-gnd{{fill:#556}}.dot-gpio{{fill:#ff9944}}
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

    // Additional GPIO pins label block
    let gpio_y = BOARD_Y + 240;
    let _ = write!(
        out,
        r#"<text x="{}" y="{}" class="pcb-sub" text-anchor="middle">GPIO</text>
<text x="{}" y="{}" class="pcb-pin" text-anchor="end">TRIG/{}</text>
<text x="{}" y="{}" class="pcb-pin" text-anchor="end">ECHO/{}</text>
<text x="{}" y="{}" class="pcb-pin" text-anchor="end">PWM/{}</text>
"#,
        cx,
        gpio_y - 6,
        BOARD_R - 7,
        gpio_y + 8,
        config.trig_pin,
        BOARD_R - 7,
        gpio_y + 22,
        config.echo_pin,
        BOARD_R - 7,
        gpio_y + 36,
        config.servo_pin,
    );

    // Devices
    let n = config.devices.len().max(1);
    let total_h = n as i32 * (DEV_H + DEV_GAP) - DEV_GAP;
    let dev_start_y = BOARD_Y + (BOARD_H - total_h) / 2;

    for (i, dev) in config.devices.iter().enumerate() {
        let dy = dev_start_y + i as i32 * (DEV_H + DEV_GAP);
        let mid_y = dy + DEV_H / 2;
        let conn = dev.kind.connection_type();

        // Control point x for Bezier curves (~60% of the way)
        let cp = BOARD_R + (DEV_X - BOARD_R) * 6 / 10;

        // Device box
        let _ = write!(
            out,
            r#"<rect x="{DEV_X}" y="{dy}" width="{DEV_W}" height="{DEV_H}" rx="4" class="dev-box"/>
<text x="{}" y="{}" class="dev-lbl">{}</text>
"#,
            DEV_X + 8,
            dy + 17,
            dev.kind.label(),
        );
        if let Some(addr) = dev.address {
            let _ = write!(
                out,
                r#"<text x="{}" y="{}" class="dev-sub">I²C addr: 0x{addr:02X}</text>
"#,
                DEV_X + 8,
                dy + 31,
            );
        }

        // VCC wire
        let y_vcc = dy + 9;
        let _ = write!(
            out,
            r#"<path class="w-vcc" d="M {BOARD_R} {P_VCC} C {cp} {P_VCC} {cp} {y_vcc} {DEV_X} {y_vcc}"/>
"#
        );

        // GND wire
        let y_gnd = dy + DEV_H - 9;
        let _ = write!(
            out,
            r#"<path class="w-gnd" d="M {BOARD_R} {P_GND} C {cp} {P_GND} {cp} {y_gnd} {DEV_X} {y_gnd}"/>
"#
        );

        match conn {
            ConnectionType::I2c => {
                let y_sda = mid_y - 7;
                let y_scl = mid_y + 5;
                let _ = write!(
                    out,
                    r#"<path class="w-sda" d="M {BOARD_R} {P_SDA} C {cp} {P_SDA} {cp} {y_sda} {DEV_X} {y_sda}"/>
<circle cx="{DEV_X}" cy="{y_sda}" r="3" class="dot-sda"/>
<path class="w-scl" d="M {BOARD_R} {P_SCL} C {cp} {P_SCL} {cp} {y_scl} {DEV_X} {y_scl}"/>
<circle cx="{DEV_X}" cy="{y_scl}" r="3" class="dot-scl"/>
"#
                );
            }
            ConnectionType::Gpio => {
                // HC-SR04: show GPIO connection
                let trig_pin = &config.trig_pin;
                let echo_pin = &config.echo_pin;
                let _ = write!(
                    out,
                    r#"<path class="w-gpio" d="M {BOARD_R} {P_GND} C {cp} {P_GND} {cp} {mid_y} {DEV_X} {mid_y}"/>
<text x="{}" y="{}" class="dev-sub">TRIG:{trig_pin} / ECHO:{echo_pin}</text>
"#,
                    DEV_X + 8,
                    dy + 44,
                );
            }
        }
    }

    // Legend at bottom — use CSS classes to avoid "# in raw string
    let leg_y = H - 10;
    let _ = write!(
        out,
        r#"<text x="{}" y="{leg_y}" class="leg" text-anchor="middle"><tspan class="dot-sda">━━</tspan> SDA <tspan dx="8" class="dot-scl">━━</tspan> SCL <tspan dx="8" class="dot-vcc">━━</tspan> VCC <tspan dx="8" class="dot-gnd">━━</tspan> GND <tspan dx="8" class="dot-gpio">━━</tspan> GPIO</text>
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
    }

    #[test]
    fn wiring_svg_contains_pin_labels() {
        let cfg = WiringConfig::from_board(BoardProfile::OriginalEsp32);
        let svg = wiring_svg(&cfg);
        assert!(svg.contains("GPIO21"), "missing SDA pin");
        assert!(svg.contains("GPIO22"), "missing SCL pin");
        assert!(svg.contains("3V3"), "missing power pin");
    }

    #[test]
    fn wiring_svg_contains_animation_css() {
        let cfg = WiringConfig::from_board(BoardProfile::OriginalEsp32);
        let svg = wiring_svg(&cfg);
        assert!(svg.contains("wiring-flow"), "missing CSS animation name");
        assert!(svg.contains("stroke-dasharray"), "missing dash array");
        assert!(svg.contains("w-sda"), "missing SDA wire class");
        assert!(svg.contains("w-scl"), "missing SCL wire class");
    }

    #[test]
    fn wiring_svg_is_valid_svg_element() {
        let cfg = WiringConfig::from_board(BoardProfile::OriginalEsp32);
        let svg = wiring_svg(&cfg);
        assert!(svg.starts_with("<svg "));
        assert!(svg.ends_with("</svg>"));
    }
}
