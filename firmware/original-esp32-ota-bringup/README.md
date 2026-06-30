# original-esp32-ota-bringup

original ESP32 向けの WiFi OTA ファームウェアです。
ESP32 が WiFi に接続してから TCP ポート 8080 で待機し、  
`scripts/flash-esp32.sh --ota <IP>` で送られてきたバイナリを  
OTA スロットに書き込んで自動再起動します。

## なぜ OTA が必要か

このファームウェアを一度 USB で焼いておけば、  
以後は USB ケーブルなしで **WiFi 経由**でファームウェアを更新できます。

```
┌─────────────────────────────────────────────────────┐
│  USB (初回のみ)                                      │
│  ./scripts/flash-esp32.sh firmware/original-esp32-ota-bringup
└────────────────────────────────────────────────────┘
          ↓ 起動後は WiFi 待機
┌─────────────────────────────────────────────────────┐
│  WiFi OTA (2回目以降)                                │
│  ./scripts/flash-esp32.sh firmware/original-esp32-climate-display \
│      --ota 192.168.1.42                              │
└─────────────────────────────────────────────────────┘
```

## 前提

| 項目 | 内容 |
|------|------|
| ボード | original ESP32 DevKitC / WROOM-32 |
| フラッシュ | 4 MB |
| toolchain | `esp` (Xtensa fork, `espup install` で導入) |
| ツール | `espflash`, `curl` |

## 依存 crate のバージョン確認

このファームウェアは `esp-hal 1.0.0` に合わせた `esp-wifi` / `smoltcp` / `esp-ota` を使います。  
ビルド前に互換バージョンを確認してください:

```bash
# esp-wifi の最新互換バージョンを確認
cargo search esp-wifi | head -5

# 必要に応じて Cargo.toml のバージョンを修正してから
cargo update
```

| crate | 推奨バージョン帯 |
|-------|----------------|
| esp-wifi | 0.13.x |
| smoltcp | 0.11.x |
| esp-ota | 0.1.x |

## 初回セットアップ — USB フラッシュ

### 1. WiFi 認証情報を環境変数で渡してビルド

```bash
cd firmware/original-esp32-ota-bringup

OTA_WIFI_SSID="MyNetwork" OTA_WIFI_PSK="MyPassword" \
    cargo build --release
```

または `~/.cargo/config.toml` や shell profile に設定しておくと便利です:

```toml
# ~/.cargo/config.toml
[env]
OTA_WIFI_SSID = "MyNetwork"
OTA_WIFI_PSK  = "MyPassword"
```

### 2. USB で初回フラッシュ

```bash
# リポジトリルートから
OTA_WIFI_SSID="MyNetwork" OTA_WIFI_PSK="MyPassword" \
    ./scripts/flash-esp32.sh firmware/original-esp32-ota-bringup
```

### 3. シリアルログで IP を確認

起動後にシリアルモニタに次のようなログが出ます:

```
[ota] WiFi started, connecting to "MyNetwork"…
[ota] connected
[ota] IP address: 192.168.1.42  OTA port: 8080
[ota] ready — waiting for firmware upload
[ota] upload with:  ./scripts/flash-esp32.sh <firmware> --ota 192.168.1.42
```

## OTA アップデート手順

IP が分かったら USB ケーブルを外して WiFi 経由で更新できます:

```bash
# climate-display を OTA で書き込む場合
./scripts/flash-esp32.sh firmware/original-esp32-climate-display \
    --ota 192.168.1.42
```

成功すると ESP32 が自動的に再起動して新しいファームウェアが起動します。

### IP アドレスを固定したい場合

ルーターの DHCP 設定で ESP32 の MAC アドレスに固定 IP を割り当てるか、  
または初回起動時のシリアルログから IP を控えておきます。

## パーティション構成

`partitions.csv` で OTA 用の二重バッファレイアウトを定義しています:

| 名前 | 種別 | オフセット | サイズ |
|------|------|-----------|--------|
| nvs | data/nvs | 0x9000 | 20 KB |
| otadata | data/ota | 0xE000 | 8 KB |
| app0 (ota_0) | app | 0x10000 | 1.9 MB |
| app1 (ota_1) | app | 0x1F0000 | 1.9 MB |
| spiffs | data | 0x3E0000 | 128 KB |

> **Note**: `espflash flash` を実行するときに `partitions.csv` を自動的に使うには  
> `espflash.toml` または `.cargo/config.toml` の `[target]` セクションで指定してください。  
> 現在は手動で `--partition-table partitions.csv` を付ける必要があります。

## OTA プロトコル仕様

このファームウェアは HTTP/1.0 の最小サブセットを実装しています:

```
POST /ota HTTP/1.0\r\n
Content-Type: application/octet-stream\r\n
Content-Length: <size>\r\n
\r\n
<firmware binary>
```

レスポンス:
- `200 OK` — 書き込み成功、ESP32 が再起動
- `500 Internal Server Error` — 書き込み失敗

curl でも直接送信できます:

```bash
curl -X POST http://192.168.1.42:8080/ota \
    --header "Content-Type: application/octet-stream" \
    --data-binary @target/xtensa-esp32-none-elf/release/my-firmware.bin
```

## トラブルシューティング

### `OTA_WIFI_SSID` が未設定でビルドエラー

```
error: environment variable `OTA_WIFI_SSID` not defined
```

→ ビルド時に環境変数を渡してください:
```bash
OTA_WIFI_SSID="..." OTA_WIFI_PSK="..." cargo build --release
```

### WiFi に接続できない

- SSID / PSK が正しいか確認してください
- 2.4 GHz 帯のみ対応です (5 GHz 非対応)
- AP の MAC アドレスフィルタリングを確認してください

### OTA 転送は成功するが起動しない

- `partitions.csv` の OTA パーティションレイアウトが正しいか確認してください
- 書き込む firmware が `esp_app_desc!()` マクロを呼び出しているか確認してください

### `esp-wifi` のビルドエラー

esp-wifi は esp-hal と密に結合しています。バージョン不一致の場合:

```bash
cargo update esp-wifi
# それでも失敗する場合は esp-hal のバージョンと揃える
```
