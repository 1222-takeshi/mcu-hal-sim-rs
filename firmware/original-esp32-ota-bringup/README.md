# original-esp32-ota-bringup

original ESP32 向けの WiFi OTA 受信ファームウェアです。
ESP-IDF (`esp-idf-svc`) の WiFi / OTA API を使い、ESP32 が WiFi に接続してから TCP ポート 8080 で待機します。
`scripts/flash-esp32.sh --ota <IP>` で送られてきたアプリバイナリを inactive OTA slot に書き込み、成功時に `200 OK` を返して再起動します。
OTA 書き込みは `X-OTA-Token` ヘッダで共有トークンを照合します。
この bring-up は trusted LAN 向けの最小構成で、通信は平文 HTTP です。

## 前提

| 項目 | 内容 |
|------|------|
| ボード | original ESP32 DevKitC / WROOM-32 |
| フラッシュ | 4 MB |
| Rust toolchain | `esp` (`espup install` で導入) |
| linker helper | `ldproxy` (`cargo install ldproxy`) |
| ツール | `espflash`, `curl` |

## 初回 USB フラッシュ

WiFi 認証情報と OTA 共有トークンは必ず環境変数で渡してください。ソースコードやコミットには直書きしません。

```bash
OTA_WIFI_SSID="MyNetwork" OTA_WIFI_PSK="MyPassword" OTA_AUTH_TOKEN="change-me" \
    ./scripts/flash-esp32.sh firmware/original-esp32-ota-bringup /dev/cu.usbserial-0001
```

起動後、シリアルログに次の形式で IP が表示されます。

```text
[ota] WiFi started, connecting to "MyNetwork"
[ota] connected
[ota] IP address: 192.168.1.42  OTA port: 8080
[ota] ready - waiting for firmware upload
[ota] upload with:  ./scripts/flash-esp32.sh <firmware> --ota 192.168.1.42
```

## OTA アップデート

表示された IP 宛に既存スクリプトで送信します。

```bash
OTA_AUTH_TOKEN="change-me" \
./scripts/flash-esp32.sh firmware/original-esp32-climate-display \
    --ota 192.168.1.42
```

成功すると ESP32 は `OTA OK` を返し、自動的に再起動します。

## ビルド確認

認証情報の値はビルド時に埋め込まれるため、ローカル確認でもダミー値が必要です。

```bash
cd firmware/original-esp32-ota-bringup
OTA_WIFI_SSID=dummy OTA_WIFI_PSK=dummy OTA_AUTH_TOKEN=dummy cargo check --release
OTA_WIFI_SSID=dummy OTA_WIFI_PSK=dummy OTA_AUTH_TOKEN=dummy cargo build --release
```

## パーティション構成

`espflash.toml` で `partitions.csv` を指定しています。
ESP-IDF の rollback は、OTA 先 firmware 側にも valid mark が必要になるため、この段階では無効のままにしています。

| 名前 | 種別 | オフセット | サイズ |
|------|------|-----------|--------|
| nvs | data/nvs | 0x9000 | 20 KB |
| otadata | data/ota | 0xE000 | 8 KB |
| app0 (ota_0) | app | 0x10000 | 1.875 MB |
| app1 (ota_1) | app | 0x1F0000 | 1.875 MB |
| spiffs | data/spiffs | 0x3E0000 | 128 KB |

## OTA プロトコル仕様

```text
POST /ota HTTP/1.0
Content-Type: application/octet-stream
X-OTA-Token: <token>
Content-Length: <size>

<firmware binary>
```

レスポンス:

- `200 OK`: 書き込み成功、ESP32 が再起動
- `400 Bad Request`: `POST /ota HTTP/1.x` ではない
- `401 Unauthorized`: `X-OTA-Token` がない、または不一致
- `431 Request Header Fields Too Large`: HTTP header が上限を超過
- `411 Length Required`: `Content-Length` がない、または不正
- `413 Payload Too Large`: OTA slot サイズを超過
- `500 Internal Server Error`: OTA 書き込み失敗

## トラブルシューティング

### `ldproxy` が見つからない

```text
error: linker `ldproxy` not found
```

次を実行してください。

```bash
cargo install ldproxy
```

### WiFi に接続できない

- SSID / PSK が正しいか確認してください
- OTA 送信時の `OTA_AUTH_TOKEN` が、OTA 受信ファームをビルドしたときの値と一致するか確認してください
- original ESP32 は 2.4 GHz WiFi のみ対応です
- AP の MAC アドレスフィルタリングや隔離設定を確認してください

### OTA 転送は成功するが次のファームが起動しない

- OTA 先の firmware が ESP32 用にビルドされているか確認してください
- `partitions.csv` の OTA slot サイズを超えていないか確認してください
- 必要に応じて USB で `original-esp32-climate-display` を再フラッシュして復旧してください
