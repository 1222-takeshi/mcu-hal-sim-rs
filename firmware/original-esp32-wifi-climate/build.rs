//! ビルド設定を `.env` から読み込み、コンパイル時 env (`env!`) として注入する。
//!
//! `.env` が存在すれば各 `KEY=VALUE` を `cargo:rustc-env` に転記する。
//! `.env` が無い場合は何もしないので、環境変数を直接渡す運用も引き続き可能。
//! 秘匿情報 (WiFi パスワード等) は `.env` (gitignore 済み) に置き、git 履歴へ
//! 残さないこと。

use std::fs;
use std::path::Path;

/// `.env` が無くても環境変数で渡せるよう、変更検知しておくキー。
const KNOWN_KEYS: &[&str] = &[
    "WIFI_SSID",
    "WIFI_PSK",
    "RASPI_IP",
    "DEVICE_ID",
    "OTA_AUTH_TOKEN",
];

fn main() {
    println!("cargo:rerun-if-changed=.env");
    for key in KNOWN_KEYS {
        println!("cargo:rerun-if-env-changed={key}");
    }

    let contents = match fs::read_to_string(Path::new(".env")) {
        Ok(c) => c,
        Err(_) => return, // .env なし: 直接渡された環境変数に委ねる
    };

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let mut value = value.trim();
        // 任意のクォートを剥がす ("value" / 'value')
        if value.len() >= 2
            && ((value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\'')))
        {
            value = &value[1..value.len() - 1];
        }
        if !key.is_empty() {
            println!("cargo:rustc-env={key}={value}");
        }
    }
}
