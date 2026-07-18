# original-esp32-wifi-climate

original ESP32 用の運用ファームウェア。WiFi 接続 → BME280 温湿度読み取り →
LCD1602 表示 → Raspberry Pi IoT サーバーへ 30 秒ごとに HTTP POST する。
バックグラウンドで管理用 HTTP サーバー (TCP 8080) も稼働し、`POST /switch` で
OTA updater パーティションへ退避できる。

## 設定 (`.env`)

WiFi 認証情報やサーバー情報などの秘匿・環境依存の値は `.env` で管理する。
**`.env` は gitignore 済みで、git 履歴には決して入れないこと。**

```sh
cp .env.example .env
# .env を編集して自分の値を入れる
```

| キー             | 説明                                                              |
| ---------------- | ----------------------------------------------------------------- |
| `WIFI_SSID`      | 接続する WiFi の SSID                                              |
| `WIFI_PSK`       | WiFi パスワード                                                   |
| `RASPI_IP`       | 送信先 Raspberry Pi サーバーの IP (`POST /api/sensors/reading`)   |
| `DEVICE_ID`      | サーバーへ報告するデバイス識別子                                  |
| `OTA_AUTH_TOKEN` | `POST /switch` を認可する共有シークレット (updater 側と一致させる) |

`build.rs` が `.env` を読み込み、コンパイル時 env (`env!`) として注入する。
`.env` を使わず環境変数を直接渡すこともできる:

```sh
WIFI_SSID=<ssid> WIFI_PSK=<psk> RASPI_IP=<ip> DEVICE_ID=<id> OTA_AUTH_TOKEN=<token> \
    cargo build --release
```

## ビルド / フラッシュ

```sh
cargo build --release
# USB 経由でフラッシュ (初回)
./scripts/flash-esp32.sh firmware/original-esp32-wifi-climate
```

## OTA アップデート

WiFi 稼働中のフラッシュ書き込みは esp-wifi と競合してハングするため、OTA 本体は
ESP-IDF ベースの updater (`firmware/original-esp32-ota-bringup`) に委譲する。

1. 稼働中ファームへ認可付きで退避要求を送る:

   ```sh
   curl -X POST -H "X-OTA-Token: <OTA_AUTH_TOKEN>" http://<esp32-ip>:8080/switch
   ```

   `X-OTA-Token` が `OTA_AUTH_TOKEN` と一致しない場合は `401 Unauthorized` で拒否される
   (同一 LAN 上の第三者による無認証ファーム上書きを防ぐ)。
2. ブートローダが updater (app0) を起動する。
3. updater の `POST /ota` へ新ファームを送ると app1 に書き込み、再起動して新ファームが稼働する。

## セキュリティ上の注意

- `.env` と実 `OTA_AUTH_TOKEN` は秘匿する。トークンは十分な長さのランダム文字列にすること。
- `/switch` は共有シークレット認証のみで、トランスポート暗号化はしていない。信頼できる
  LAN 内での運用を前提とする。
