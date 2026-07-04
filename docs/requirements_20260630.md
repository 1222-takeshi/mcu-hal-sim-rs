# ESP32 OTA 受信ファーム ESP-IDF 化 要件定義

## 目的

- original ESP32 を USB で初回書き込み後、WiFi 経由で firmware を OTA 更新できる状態にする。
- 既存の no_std / esp-hal / esp-wifi / esp-storage の組み合わせで発生していたリンク衝突・起動時 panic を避けるため、OTA 受信ファームを ESP-IDF (`esp-idf-svc`) 前提に切り替える。

## 成功基準

- `firmware/original-esp32-ota-bringup` が `xtensa-esp32-espidf` target で release build できる。
- 起動後に WiFi station として接続し、シリアルログへ DHCP IP と OTA port を表示する。
- `scripts/flash-esp32.sh <firmware> --ota <IP[:PORT]>` からの `POST /ota` を受け、inactive OTA slot に書き込む。
- 成功時は `200 OK` を返してから reboot する。
- WiFi SSID / PSK と OTA 共有トークンは環境変数でのみ渡し、ソース・ドキュメント・コミットに実値を含めない。

## I/F

| 項目 | 内容 |
|------|------|
| WiFi 認証情報 | `OTA_WIFI_SSID`, `OTA_WIFI_PSK` |
| OTA 認証 | `OTA_AUTH_TOKEN` を `X-OTA-Token` HTTP header と照合 |
| HTTP endpoint | `POST /ota HTTP/1.x` |
| Port | `8080` |
| Body | `application/octet-stream` の ESP32 app image |
| 正常レスポンス | `200 OK`, body `OTA OK\n` |
| エラーレスポンス | `400`, `401`, `411`, `413`, `500` |

## 設計

- WiFi: `EspWifi` を `BlockingWifi` で wrap し、`start()` / `connect()` / `wait_netif_up()` で DHCP 完了まで待つ。
- OTA: `EspOta::new()` で OTA singleton を取得し、起動時に `mark_running_slot_valid()` を試行する。
- rollback: OTA 先 firmware 側にも valid mark が必要になるため、この段階では無効のままとする。
- 認証: `X-OTA-Token` の値がビルド時の `OTA_AUTH_TOKEN` と一致する場合のみ OTA 書き込みを受け付ける。
- 防御: HTTP header は 1 行 512 byte、総量 2048 byte を上限にする。
- 書き込み: `Content-Length` を `initiate_update_with_known_size()` に渡し、512 byte buffer で逐次 `write()` する。
  - 実機検証で 4096 byte の stack buffer は ESP-IDF main task の stack overflow を起こしたため、stack 使用量を抑える。
- reboot: `200 OK` を `flush()` した後、短時間待って `esp_idf_svc::hal::reset::restart()` を呼ぶ。
- partition: `espflash.toml` から既存 `partitions.csv` を使用する。

## テスト観点

| 観点 | ケース | 期待結果 |
|------|--------|----------|
| 正常 | `POST /ota` + valid `Content-Length` + slot 内サイズ | OTA 書き込み完了、`200 OK`、reboot |
| 異常 | `GET /ota` または path 不一致 | `400 Bad Request` |
| 異常 | `X-OTA-Token` なし/不一致 | `401 Unauthorized` |
| 異常 | HTTP header が上限超過 | `431 Request Header Fields Too Large` |
| 異常 | `Content-Length` なし/不正 | `411 Length Required` |
| 境界 | `Content-Length = 0` または `0x1E0000` 超過 | `413 Payload Too Large` |
| 異常 | 転送途中 EOF | `500 Internal Server Error` |
| セキュリティ | SSID/PSK/token の直書きなし | 環境変数以外に秘密情報が残らない |

## 検証結果

- `OTA_WIFI_SSID=dummy OTA_WIFI_PSK=dummy OTA_AUTH_TOKEN=dummy cargo check --release`: 成功
- `OTA_WIFI_SSID=dummy OTA_WIFI_PSK=dummy OTA_AUTH_TOKEN=dummy cargo build --release`: 成功
- 実機 USB 初回書き込み（ESP-IDF OTA 経路確認時）: 成功
  - `espflash write-bin --port <serial-port> 0x10000 <ota-receiver-app-image>`
  - シリアルログで `[ota] booting ESP-IDF OTA receiver` と `[ota] IP address: <ip>  OTA port: 8080` を確認
- 実機 OTA 書き込み（ESP-IDF OTA 経路確認時）: 成功
  - `scripts/flash-esp32.sh firmware/original-esp32-climate-display --ota <ip>`
  - `OTA OK` / HTTP 200 を確認
  - 再起動後に `Loaded app from partition at offset 0x1f0000` と `original ESP32 climate display started` を確認
- rollback 設定: OTA 先 firmware の互換性を優先し、この段階では無効化
- `X-OTA-Token` 導入後の実機再書き込み: ローカル承認枠の制約により未実施
