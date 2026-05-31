use std::io::Write as _;
use std::net::TcpStream;

// ── Firmware Flash targets ──────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum BoardKind {
    Esp32,
    M5StickC,
    ArduinoNano,
    RaspberryPiPico,
}

impl BoardKind {
    pub(super) fn label(self) -> &'static str {
        match self {
            BoardKind::Esp32 => "ESP32",
            BoardKind::M5StickC => "M5StickC",
            BoardKind::ArduinoNano => "Arduino Nano",
            BoardKind::RaspberryPiPico => "Raspberry Pi Pico",
        }
    }
}

pub(super) struct FlashTarget {
    pub id: &'static str,
    pub label: &'static str,
    pub firmware_dir: &'static str,
    pub binary_name: &'static str,
    pub target_triple: &'static str,
    pub board: BoardKind,
}

pub(super) fn flash_targets() -> &'static [FlashTarget] {
    &[
        // ── ESP32 ──────────────────────────────────────────────────────────
        FlashTarget {
            id: "esp32-climate-display",
            label: "BME280 + LCD1602 (climate display)",
            firmware_dir: "firmware/original-esp32-climate-display",
            binary_name: "original-esp32-climate-display",
            target_triple: "xtensa-esp32-none-elf",
            board: BoardKind::Esp32,
        },
        FlashTarget {
            id: "esp32-robot-base",
            label: "Robot base (servo + motors)",
            firmware_dir: "firmware/original-esp32-robot-base",
            binary_name: "original-esp32-robot-base",
            target_triple: "xtensa-esp32-none-elf",
            board: BoardKind::Esp32,
        },
        FlashTarget {
            id: "esp32-bringup",
            label: "Bringup (GPIO / I2C check)",
            firmware_dir: "firmware/original-esp32-bringup",
            binary_name: "original-esp32-bringup",
            target_triple: "xtensa-esp32-none-elf",
            board: BoardKind::Esp32,
        },
        // ── M5StickC ───────────────────────────────────────────────────────
        FlashTarget {
            id: "m5stickc-bringup",
            label: "Bringup (GPIO / I2C check)",
            firmware_dir: "firmware/m5stickc-bringup",
            binary_name: "m5stickc-bringup",
            target_triple: "xtensa-esp32-none-elf",
            board: BoardKind::M5StickC,
        },
        // ── Arduino Nano ───────────────────────────────────────────────────
        FlashTarget {
            id: "arduino-nano-climate-display",
            label: "BME280 + LCD1602 (climate display)",
            firmware_dir: "firmware/arduino-nano-climate-display",
            binary_name: "arduino-nano-climate-display",
            target_triple: "avr-none",
            board: BoardKind::ArduinoNano,
        },
        FlashTarget {
            id: "arduino-nano-bringup",
            label: "Bringup (GPIO / I2C check)",
            firmware_dir: "firmware/arduino-nano-bringup",
            binary_name: "arduino-nano-bringup",
            target_triple: "avr-none",
            board: BoardKind::ArduinoNano,
        },
        // ── Raspberry Pi Pico ──────────────────────────────────────────────
        FlashTarget {
            id: "raspi-pico-climate-display",
            label: "BME280 + LCD1602 (climate display)",
            firmware_dir: "firmware/raspi-pico-climate-display",
            binary_name: "raspi-pico-climate-display",
            target_triple: "thumbv6m-none-eabi",
            board: BoardKind::RaspberryPiPico,
        },
        FlashTarget {
            id: "raspi-pico-bringup",
            label: "Bringup (GPIO / I2C check)",
            firmware_dir: "firmware/raspi-pico-bringup",
            binary_name: "raspi-pico-bringup",
            target_triple: "thumbv6m-none-eabi",
            board: BoardKind::RaspberryPiPico,
        },
    ]
}

/// ~/.rustup/toolchains/esp/xtensa-esp-elf/<ver>/xtensa-esp-elf/bin を探す。
/// PATH に既に入っていれば None を返しても問題ない。
pub(super) fn find_xtensa_gcc_bin_path() -> Option<std::path::PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let base = std::path::PathBuf::from(&home).join(".rustup/toolchains/esp/xtensa-esp-elf");
    std::fs::read_dir(&base).ok()?.find_map(|entry| {
        let bin = entry.ok()?.path().join("xtensa-esp-elf/bin");
        if bin.is_dir() {
            Some(bin)
        } else {
            None
        }
    })
}

/// `stream` に SSE ヘッダを書き出す。失敗時は false を返す。
pub(super) fn write_sse_header(stream: &mut TcpStream) -> bool {
    let header = "HTTP/1.1 200 OK\r\n\
        Content-Type: text/event-stream\r\n\
        Cache-Control: no-cache\r\n\
        Connection: keep-alive\r\n\
        Access-Control-Allow-Origin: *\r\n\
        \r\n";
    stream.write_all(header.as_bytes()).is_ok()
}

/// コマンドを実行し、stdout/stderr を SSE ラインとして stream に流す。
/// タイムアウト超過または stream への書き込みエラーで強制終了。
/// 戻り値: exit code (タイムアウト/エラー時は -1)
pub(super) fn stream_command(
    stream: &mut TcpStream,
    cmd: &str,
    args: &[&str],
    cwd: &std::path::Path,
    env_extra: &[(&str, &str)],
    timeout_secs: u64,
) -> i32 {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};
    use std::sync::mpsc;

    let mut command = Command::new(cmd);
    command
        .args(args)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in env_extra {
        command.env(k, v);
    }

    let mut child = match command.spawn() {
        Ok(c) => c,
        Err(e) => {
            let _ = stream.write_all(
                format!("data: [ERROR] failed to spawn `{cmd}`: {e}\n\ndata: [DONE] exit=1\n\n")
                    .as_bytes(),
            );
            return 1;
        }
    };

    let (tx, rx) = mpsc::channel::<String>();
    let tx2 = tx.clone();
    let stdout = child.stdout.take().expect("stdout piped");
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                break;
            }
        }
    });
    let stderr = child.stderr.take().expect("stderr piped");
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            if tx2.send(line).is_err() {
                break;
            }
        }
    });

    let started = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(timeout_secs);

    for line in rx {
        if started.elapsed() > timeout {
            let _ = stream.write_all(
                format!("data: [ERROR] timeout after {timeout_secs}s\n\ndata: [DONE] exit=1\n\n")
                    .as_bytes(),
            );
            let _ = child.kill();
            return -1;
        }
        // ANSI 制御コードを除去して送信
        let clean: String = line
            .chars()
            .scan(false, |in_esc, c| {
                if *in_esc {
                    *in_esc = c != 'm' && c != 'K' && c != 'J' && !c.is_alphabetic();
                    if c.is_alphabetic() {
                        *in_esc = false;
                    }
                    Some(None)
                } else if c == '\x1b' {
                    *in_esc = true;
                    Some(None)
                } else {
                    Some(Some(c))
                }
            })
            .flatten()
            .collect();
        let msg = format!("data: {}\n\n", clean.replace('\n', " "));
        if stream.write_all(msg.as_bytes()).is_err() {
            let _ = child.kill();
            return -1;
        }
    }

    child.wait().map(|s| s.code().unwrap_or(-1)).unwrap_or(-1)
}

pub(super) fn list_serial_ports() -> Vec<String> {
    let Ok(dir) = std::fs::read_dir("/dev") else {
        return vec![];
    };
    let mut ports: Vec<String> = dir
        .filter_map(|e| e.ok())
        .map(|e| e.path().to_string_lossy().to_string())
        .filter(|name| {
            // macOS: cu.usbserial*, cu.SLAB*, cu.wchusbserial*, cu.usbmodem*
            // Linux: ttyUSB*, ttyACM*
            name.contains("/cu.usbserial")
                || name.contains("/cu.SLAB")
                || name.contains("/cu.wchusbserial")
                || name.contains("/cu.usbmodem")
                || name.contains("/ttyUSB")
                || name.contains("/ttyACM")
        })
        .collect();
    ports.sort();
    ports
}

/// Convert a board string (from query param) to a `BoardKind`.
pub(super) fn board_kind_from_str(s: &str) -> BoardKind {
    match s {
        "m5stickc" => BoardKind::M5StickC,
        "arduino-nano" => BoardKind::ArduinoNano,
        "raspi-pico" => BoardKind::RaspberryPiPico,
        _ => BoardKind::Esp32,
    }
}

/// Read `.cargo/config.toml` in `dir` and extract the `target = "..."` line.
pub(super) fn detect_build_target(dir: &std::path::Path) -> Option<String> {
    let content = std::fs::read_to_string(dir.join(".cargo").join("config.toml")).ok()?;
    content
        .lines()
        .find(|l| l.trim_start().starts_with("target ="))
        .and_then(|l| l.split('"').nth(1))
        .map(String::from)
}

/// Read `Cargo.toml` in `dir` and extract the package `name = "..."` line.
pub(super) fn detect_binary_name(dir: &std::path::Path) -> Option<String> {
    let content = std::fs::read_to_string(dir.join("Cargo.toml")).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("name =") {
            return trimmed.split('"').nth(1).map(String::from);
        }
    }
    None
}

/// ターゲット指定 or 旧来の bin 指定でビルド＋書き込みを SSE ストリーム。
///
/// Query params:
///   target=<id>      (flash_targets() の id を指定)
///   port=<device>
///   board=<board>    (custom_elf/custom_dir 時のボード指定)
///   custom_elf=<abs> (ビルド済み ELF を直接 flash)
///   custom_dir=<abs> (外部 Rust プロジェクトを cargo build + flash)
///   bin=<path>       (後方互換: target 未指定時の旧 ESP32 直接 flash)
pub(super) fn handle_flash_stream(stream: &mut TcpStream, query: &str) {
    use std::process::{Command, Stdio};

    if !write_sse_header(stream) {
        return;
    }

    let parse = |prefix: &str| -> String {
        query
            .split('&')
            .find_map(|kv| kv.strip_prefix(prefix))
            .unwrap_or("")
            .replace("%2F", "/")
            .replace("%3A", ":")
            .replace("%20", " ")
    };

    let target_id = parse("target=");
    let port = parse("port=");
    let bin = parse("bin=");
    let custom_elf = parse("custom_elf=");
    let custom_dir = parse("custom_dir=");
    let board_str = parse("board=");

    // ── target= が指定されている場合: build + flash ──────────────────────────
    if !target_id.is_empty() {
        let Some(target) = flash_targets().iter().find(|t| t.id == target_id) else {
            let _ = stream.write_all(
                format!("data: [ERROR] Unknown target: {target_id}\n\ndata: [DONE] exit=1\n\n")
                    .as_bytes(),
            );
            return;
        };

        if port.is_empty() && !matches!(target.board, BoardKind::RaspberryPiPico) {
            let _ =
                stream.write_all(b"data: [ERROR] No port specified.\n\ndata: [DONE] exit=1\n\n");
            return;
        }

        let workspace = std::env::current_dir().unwrap_or_default();
        let firmware_dir = workspace.join(target.firmware_dir);

        match target.board {
            BoardKind::Esp32 | BoardKind::M5StickC => {
                // Step 1: cargo build --release (Xtensa GCC を PATH に追加)
                let _ = stream.write_all(
                    format!("data: [BUILD] Building {}...\n\n", target.label).as_bytes(),
                );

                let mut path_val = std::env::var("PATH").unwrap_or_default();
                if let Some(gcc_bin) = find_xtensa_gcc_bin_path() {
                    path_val = format!("{}:{}", gcc_bin.display(), path_val);
                }

                let build_code = stream_command(
                    stream,
                    "cargo",
                    &["build", "--release", "--color", "never"],
                    &firmware_dir,
                    &[("PATH", &path_val)],
                    300,
                );
                if build_code != 0 {
                    let _ = stream.write_all(
                        format!("data: [ERROR] Build failed (exit={build_code})\n\ndata: [DONE] exit={build_code}\n\n")
                            .as_bytes(),
                    );
                    return;
                }

                // espflash が存在するか確認
                if Command::new("espflash")
                    .arg("--version")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .is_err()
                {
                    let _ = stream.write_all(
                        b"data: [ERROR] espflash not found. Install: cargo install espflash\n\ndata: [DONE] exit=1\n\n",
                    );
                    return;
                }

                // Step 2: espflash flash --port <port> <elf>
                let elf = firmware_dir
                    .join("target")
                    .join(target.target_triple)
                    .join("release")
                    .join(target.binary_name);

                let _ = stream.write_all(b"data: [FLASH] Flashing via espflash...\n\n");
                let flash_code = stream_command(
                    stream,
                    "espflash",
                    &["flash", "--port", &port, elf.to_str().unwrap_or("")],
                    &workspace,
                    &[("PATH", &path_val)],
                    120,
                );
                let _ = stream.write_all(format!("data: [DONE] exit={flash_code}\n\n").as_bytes());
            }

            BoardKind::ArduinoNano => {
                // ravedude が存在するか確認
                if Command::new("ravedude")
                    .arg("--version")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .is_err()
                {
                    let _ = stream.write_all(
                        b"data: [ERROR] ravedude not found.\n\n\
                          data: Install: cargo install ravedude\n\n\
                          data: Docs: https://github.com/Rahix/avr-hal/tree/main/ravedude\n\n\
                          data: [DONE] exit=1\n\n",
                    );
                    return;
                }

                let _ = stream.write_all(
                    format!(
                        "data: [BUILD+FLASH] Building and flashing {} via ravedude...\n\n",
                        target.label
                    )
                    .as_bytes(),
                );

                // cargo run --release: ravedude が build 後に自動書き込み
                let exit_code = stream_command(
                    stream,
                    "cargo",
                    &["run", "--release", "--color", "never"],
                    &firmware_dir,
                    &[("RAVEDUDE_PORT", &port)],
                    300,
                );
                let _ = stream.write_all(format!("data: [DONE] exit={exit_code}\n\n").as_bytes());
            }

            BoardKind::RaspberryPiPico => {
                // elf2uf2-rs が存在するか確認
                if Command::new("elf2uf2-rs")
                    .arg("--help")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .is_err()
                {
                    let _ = stream.write_all(
                        b"data: [ERROR] elf2uf2-rs not found.\n\n\
                          data: Install: cargo install elf2uf2-rs\n\n\
                          data: [DONE] exit=1\n\n",
                    );
                    return;
                }

                // Step 1: cargo build --release
                let _ = stream.write_all(
                    format!("data: [BUILD] Building {}...\n\n", target.label).as_bytes(),
                );
                let build_code = stream_command(
                    stream,
                    "cargo",
                    &["build", "--release", "--color", "never"],
                    &firmware_dir,
                    &[],
                    300,
                );
                if build_code != 0 {
                    let _ = stream.write_all(
                        format!("data: [ERROR] Build failed (exit={build_code})\n\ndata: [DONE] exit={build_code}\n\n")
                            .as_bytes(),
                    );
                    return;
                }

                // Step 2: elf2uf2-rs -d <elf>  (-d = deploy, waits for BOOTSEL mode)
                let elf = firmware_dir
                    .join("target")
                    .join(target.target_triple)
                    .join("release")
                    .join(target.binary_name);

                let _ = stream.write_all(
                    b"data: [FLASH] Waiting for Pico in BOOTSEL mode (hold BOOTSEL then plug USB)...\n\n",
                );
                let flash_code = stream_command(
                    stream,
                    "elf2uf2-rs",
                    &["-d", elf.to_str().unwrap_or("")],
                    &workspace,
                    &[],
                    120,
                );
                let _ = stream.write_all(format!("data: [DONE] exit={flash_code}\n\n").as_bytes());
            }
        }
        return;
    }

    // ── custom_elf= : flash a pre-built ELF ─────────────────────────────────
    if !custom_elf.is_empty() {
        let board = board_kind_from_str(&board_str);
        let elf_path = std::path::Path::new(&custom_elf);
        if !elf_path.is_absolute() {
            let _ = stream.write_all(
                b"data: [ERROR] Path must be absolute (e.g. /home/user/firmware.elf).\n\n\
                  data: [DONE] exit=1\n\n",
            );
            return;
        }
        if !elf_path.exists() {
            let _ = stream.write_all(
                format!("data: [ERROR] ELF not found: {custom_elf}\n\ndata: [DONE] exit=1\n\n")
                    .as_bytes(),
            );
            return;
        }
        let workspace = std::env::current_dir().unwrap_or_default();
        match board {
            BoardKind::Esp32 | BoardKind::M5StickC => {
                if port.is_empty() {
                    let _ = stream
                        .write_all(b"data: [ERROR] No port specified.\n\ndata: [DONE] exit=1\n\n");
                    return;
                }
                if Command::new("espflash")
                    .arg("--version")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .is_err()
                {
                    let _ = stream.write_all(
                        b"data: [ERROR] espflash not found. Install: cargo install espflash\n\n\
                          data: [DONE] exit=1\n\n",
                    );
                    return;
                }
                let _ = stream.write_all(b"data: [FLASH] Flashing via espflash...\n\n");
                let code = stream_command(
                    stream,
                    "espflash",
                    &["flash", "--port", &port, &custom_elf],
                    &workspace,
                    &[],
                    120,
                );
                let _ = stream.write_all(format!("data: [DONE] exit={code}\n\n").as_bytes());
            }
            BoardKind::ArduinoNano => {
                if port.is_empty() {
                    let _ = stream
                        .write_all(b"data: [ERROR] No port specified.\n\ndata: [DONE] exit=1\n\n");
                    return;
                }
                let _ = stream.write_all(b"data: [FLASH] Flashing via avrdude...\n\n");
                let flash_arg = format!("flash:w:{custom_elf}:e");
                let code = stream_command(
                    stream,
                    "avrdude",
                    &[
                        "-p", "m328p", "-c", "arduino", "-P", &port, "-b", "115200", "-U",
                        &flash_arg,
                    ],
                    &workspace,
                    &[],
                    120,
                );
                let _ = stream.write_all(format!("data: [DONE] exit={code}\n\n").as_bytes());
            }
            BoardKind::RaspberryPiPico => {
                if Command::new("elf2uf2-rs")
                    .arg("--help")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .is_err()
                {
                    let _ = stream.write_all(
                        b"data: [ERROR] elf2uf2-rs not found. Install: cargo install elf2uf2-rs\n\n\
                          data: [DONE] exit=1\n\n",
                    );
                    return;
                }
                let _ = stream.write_all(
                    b"data: [FLASH] Waiting for Pico in BOOTSEL mode (hold BOOTSEL then plug USB)...\n\n",
                );
                let code = stream_command(
                    stream,
                    "elf2uf2-rs",
                    &["-d", &custom_elf],
                    &workspace,
                    &[],
                    120,
                );
                let _ = stream.write_all(format!("data: [DONE] exit={code}\n\n").as_bytes());
            }
        }
        return;
    }

    // ── custom_dir= : cargo build + flash an external Rust project ───────────
    if !custom_dir.is_empty() {
        let board = board_kind_from_str(&board_str);
        let dir_path = std::path::PathBuf::from(&custom_dir);
        if !dir_path.is_absolute() {
            let _ = stream.write_all(
                b"data: [ERROR] Path must be absolute (e.g. /home/user/my-firmware).\n\n\
                  data: [DONE] exit=1\n\n",
            );
            return;
        }
        if !dir_path.exists() {
            let _ = stream.write_all(
                format!(
                    "data: [ERROR] Directory not found: {custom_dir}\n\ndata: [DONE] exit=1\n\n"
                )
                .as_bytes(),
            );
            return;
        }
        let workspace = std::env::current_dir().unwrap_or_default();
        match board {
            BoardKind::Esp32 | BoardKind::M5StickC => {
                if port.is_empty() {
                    let _ = stream
                        .write_all(b"data: [ERROR] No port specified.\n\ndata: [DONE] exit=1\n\n");
                    return;
                }
                let _ = stream.write_all(b"data: [BUILD] Building (cargo build --release)...\n\n");
                let mut path_val = std::env::var("PATH").unwrap_or_default();
                if let Some(gcc_bin) = find_xtensa_gcc_bin_path() {
                    path_val = format!("{}:{}", gcc_bin.display(), path_val);
                }
                let build_code = stream_command(
                    stream,
                    "cargo",
                    &["build", "--release", "--color", "never"],
                    &dir_path,
                    &[("PATH", &path_val)],
                    300,
                );
                if build_code != 0 {
                    let _ = stream.write_all(
                        format!("data: [ERROR] Build failed (exit={build_code})\n\ndata: [DONE] exit={build_code}\n\n")
                            .as_bytes(),
                    );
                    return;
                }
                if Command::new("espflash")
                    .arg("--version")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .is_err()
                {
                    let _ = stream.write_all(
                        b"data: [ERROR] espflash not found. Install: cargo install espflash\n\n\
                          data: [DONE] exit=1\n\n",
                    );
                    return;
                }
                let target_triple = detect_build_target(&dir_path)
                    .unwrap_or_else(|| "xtensa-esp32-none-elf".to_string());
                let bin_name =
                    detect_binary_name(&dir_path).unwrap_or_else(|| "firmware".to_string());
                let elf = dir_path
                    .join("target")
                    .join(&target_triple)
                    .join("release")
                    .join(&bin_name);
                let elf_str = elf.to_string_lossy().to_string();
                if !elf.exists() {
                    let _ = stream.write_all(
                        format!(
                            "data: [ERROR] Built ELF not found: {elf_str}\n\n\
                             data: Hint: check binary name in Cargo.toml and .cargo/config.toml\n\n\
                             data: [DONE] exit=1\n\n"
                        )
                        .as_bytes(),
                    );
                    return;
                }
                let _ = stream.write_all(b"data: [FLASH] Flashing via espflash...\n\n");
                let code = stream_command(
                    stream,
                    "espflash",
                    &["flash", "--port", &port, &elf_str],
                    &workspace,
                    &[("PATH", &path_val)],
                    120,
                );
                let _ = stream.write_all(format!("data: [DONE] exit={code}\n\n").as_bytes());
            }
            BoardKind::ArduinoNano => {
                if port.is_empty() {
                    let _ = stream
                        .write_all(b"data: [ERROR] No port specified.\n\ndata: [DONE] exit=1\n\n");
                    return;
                }
                if Command::new("ravedude")
                    .arg("--version")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .is_err()
                {
                    let _ = stream.write_all(
                        b"data: [ERROR] ravedude not found. Install: cargo install ravedude\n\n\
                          data: [DONE] exit=1\n\n",
                    );
                    return;
                }
                let _ = stream
                    .write_all(b"data: [BUILD+FLASH] Building and flashing via ravedude...\n\n");
                let code = stream_command(
                    stream,
                    "cargo",
                    &["run", "--release", "--color", "never"],
                    &dir_path,
                    &[("RAVEDUDE_PORT", &port)],
                    300,
                );
                let _ = stream.write_all(format!("data: [DONE] exit={code}\n\n").as_bytes());
            }
            BoardKind::RaspberryPiPico => {
                if Command::new("elf2uf2-rs")
                    .arg("--help")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .is_err()
                {
                    let _ = stream.write_all(
                        b"data: [ERROR] elf2uf2-rs not found. Install: cargo install elf2uf2-rs\n\n\
                          data: [DONE] exit=1\n\n",
                    );
                    return;
                }
                let _ = stream.write_all(b"data: [BUILD] Building (cargo build --release)...\n\n");
                let build_code = stream_command(
                    stream,
                    "cargo",
                    &["build", "--release", "--color", "never"],
                    &dir_path,
                    &[],
                    300,
                );
                if build_code != 0 {
                    let _ = stream.write_all(
                        format!("data: [ERROR] Build failed (exit={build_code})\n\ndata: [DONE] exit={build_code}\n\n")
                            .as_bytes(),
                    );
                    return;
                }
                let target_triple = detect_build_target(&dir_path)
                    .unwrap_or_else(|| "thumbv6m-none-eabi".to_string());
                let bin_name =
                    detect_binary_name(&dir_path).unwrap_or_else(|| "firmware".to_string());
                let elf = dir_path
                    .join("target")
                    .join(&target_triple)
                    .join("release")
                    .join(&bin_name);
                let elf_str = elf.to_string_lossy().to_string();
                if !elf.exists() {
                    let _ = stream.write_all(
                        format!(
                            "data: [ERROR] Built ELF not found: {elf_str}\n\n\
                             data: Hint: check binary name in Cargo.toml and .cargo/config.toml\n\n\
                             data: [DONE] exit=1\n\n"
                        )
                        .as_bytes(),
                    );
                    return;
                }
                let _ = stream.write_all(
                    b"data: [FLASH] Waiting for Pico in BOOTSEL mode (hold BOOTSEL then plug USB)...\n\n",
                );
                let code = stream_command(
                    stream,
                    "elf2uf2-rs",
                    &["-d", &elf_str],
                    &workspace,
                    &[],
                    120,
                );
                let _ = stream.write_all(format!("data: [DONE] exit={code}\n\n").as_bytes());
            }
        }
        return;
    }

    // ── 後方互換: bin= or port= のみの旧 ESP32 直接 flash ───────────────────
    if Command::new("espflash")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_err()
    {
        let _ = stream.write_all(
            b"data: [ERROR] espflash not found.\n\n\
              data: Install: cargo install espflash\n\n\
              data: [DONE] exit=1\n\n",
        );
        return;
    }

    if port.is_empty() {
        let _ = stream.write_all(b"data: [ERROR] No port specified.\n\ndata: [DONE] exit=1\n\n");
        return;
    }

    let workspace = std::env::current_dir().unwrap_or_default();
    let mut args = vec!["flash", "--port", &port];
    if !bin.is_empty() {
        args.push(&bin);
    }
    let exit_code = stream_command(stream, "espflash", &args, &workspace, &[], 120);
    let _ = stream.write_all(format!("data: [DONE] exit={exit_code}\n\n").as_bytes());
}
