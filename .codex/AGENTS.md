# AGENTS.md - mcu-hal-sim-rs (Codex 向け補足)

このファイルは Codex で `mcu-hal-sim-rs` を扱う際の補足メモです。開発ルール・ディレクトリ構成・
ビルド/テスト/lint コマンド・sim-to-real 契約・PR/CI 運用方針の正本は `CLAUDE.md` です。
本ファイルには `CLAUDE.md` に無い、Codex 運用上の補足のみを記載します。

## 前提

- プロジェクト概要・スコープ方針・実装ルール・テスト方針・PR/CI 運用は `CLAUDE.md` を参照してください。
- 使用言語は Rust。ホスト環境は macOS (Apple Silicon) / Windows / Ubuntu Linux を想定し、まず PC
  シミュレータ向けバイナリ（`platform-pc-sim`）を優先して動作確認します。

## Codex 固有の運用メモ

- コミットメッセージと PR タイトルは英語、PR の説明文は日本語で書く。
- PR の説明文には実行したテストコマンド（例: `cargo test` / `cargo run -p platform-pc-sim` など）を
  必ず明記する。
- GitHub 連携（`git push` / Pull Request / Issue 作成）は、原則として `scripts/gh-workflow.sh`
  （`push` / `pr` / `issue` サブコマンド）経由で行う。
- 実機 bring-up や `espflash` 実行手順を提案する際は、ホスト OS として macOS も必ず考慮する。
  Windows の `COMx` だけを前提にせず、macOS / Linux の native serial device path を優先し、
  WSL2 + Windows 経由は代替経路として扱う。

## ESP32 OTA receiver with ESP-IDF（追記日: 2026-06-30）

- **概要**: `firmware/original-esp32-ota-bringup` は OTA 受信ファームとして `esp-idf-svc` ベースの
  ESP-IDF std 構成を優先する。
- **詳細**: target は `xtensa-esp32-espidf`、linker は `ldproxy`、`build-std = ["std", "panic_abort"]`、
  `build.rs` では `embuild::espidf::sysenv::output()` を使う。`espflash.toml` で `partitions.csv` を
  指定し、USB 初回書込み時にも OTA partition table を使う。
- **適用条件**: original ESP32 で WiFi OTA 受信、NVS、OTA slot 切替を扱う firmware を作る場合。
- **例**: `OTA_WIFI_SSID=dummy OTA_WIFI_PSK=dummy OTA_AUTH_TOKEN=dummy cargo build --release`

## ESP32 OTA parser のホストテスト（追記日: 2026-07-11）

- **概要**: ESP-IDF に依存しない OTA HTTP 入力検証は `original-esp32-ota-bringup/ota-http` の独立
  workspace に分離し、実機や ESP toolchain なしでテストする。
- **詳細**: firmware 側は path dependency として同じ parser を使用する。親 workspace では
  `exclude = ["ota-http"]` を指定し、nested workspace の競合と ESP-IDF 依存の host 解決を避ける。
- **適用条件**: ESP-IDF firmware の純粋な入力検証を、root workspace と独立して CI 実行する場合。
- **例**: `cargo test --manifest-path firmware/original-esp32-ota-bringup/ota-http/Cargo.toml`
