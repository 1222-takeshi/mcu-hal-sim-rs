# Device Dashboard ガイド

`device-dashboard-web` は、PC simulator 上のセンサー値・アクチュエーター状態・I2C 配線をブラウザでリアルタイム確認するためのローカル Web ダッシュボードです。

---

## 目次

1. [クイックスタート](#クイックスタート)
2. [パネル説明](#パネル説明)
3. [Board / Profile / Device 選択](#board--profile--device-選択)
4. [Wiring Diagram](#wiring-diagram)
5. [REST API](#rest-api)
6. [Simulator → Dashboard → 実機 (sim-to-real)](#simulator--dashboard--実機-sim-to-real)
7. [ブラウザ対応状況](#ブラウザ対応状況)
8. [トラブルシューティング](#トラブルシューティング)

---

## クイックスタート

```bash
# デフォルト: ESP32 board / Full profile / port 7878
cargo run -p platform-pc-sim --bin device-dashboard-web

# Arduino Nano board / port 7878
cargo run -p platform-pc-sim --bin device-dashboard-web -- nano 7878
```

ブラウザで `http://127.0.0.1:7878` を開くと、センサー値が SSE でリアルタイム更新されます。

---

## パネル説明

### ステータスバー

| 表示 | 意味 |
|------|------|
| `Online · updated Xs ago` | SSE 接続中、最終更新時刻 |
| `Paused` | ユーザーが一時停止中（右上の Pause ボタン） |
| `Error ×N: <message>` | wiring 取得エラー等、N 回連続エラー |

### Simulator Controls

- **Pause / Resume**: センサー値の UI 更新を一時停止します。SSE ストリームは継続します。

### Climate (BME280)

温度 / 湿度 / 気圧のリアルタイム値。BME280 が有効な場合のみ表示されます。

### LCD Display (LCD1602)

仮想 LCD 1602 の物理フレーム (2 行 × 16 文字) を表示します。

### Distance (HC-SR04)

超音波距離センサーの距離値 (mm)。HC-SR04 が有効な場合のみ表示されます。

### IMU (MPU-6050)

加速度 (mg) と角速度 (mdps) の 3 軸値。MPU-6050 が有効な場合のみ表示されます。

### Servo Motor

サーボの現在角度 (degrees)。Servo が有効な場合のみ表示されます。

### Motor Driver (L298N)

左右モーターの方向と duty % を表示します。L298N が有効な場合のみ表示されます。

### Light (BH1750)

照度センサーの lux 値。BH1750 が有効な場合のみ表示されます。

### Gas (SGP30)

CO₂ / VOC センサーの値。SGP30 が有効な場合のみ表示されます。

### RTC (DS3231)

リアルタイムクロックの日時。DS3231 が有効な場合のみ表示されます。

### ToF Distance (VL53L0X)

ToF レーザー距離センサーの値。VL53L0X が有効な場合のみ表示されます。

---

## Board / Profile / Device 選択

### Board

| スラッグ | ボード |
|---------|------|
| `esp32` (デフォルト) | Original ESP32 (GPIO21=SDA, GPIO22=SCL) |
| `nano` | Arduino Nano (A4=SDA, A5=SCL) |

Board を切り替えると Wiring Diagram のピン番号が更新されます。

### Sensor Profile

プリセット構成でデバイスの選択状態を一括変更します。

| スラッグ | 表示名 | 含まれるデバイス |
|---------|--------|--------------|
| `full` | Full (all devices) | BME280 / MPU-6050 / LCD1602 / BH1750 / DS3231 / SGP30 / VL53L0X / Servo / L298N / HC-SR04 / ESP32-CAM |
| `climate` | Climate Station | BME280 / BH1750 / SGP30 / DS3231 / LCD1602 |
| `robot` | Robot Base | MPU-6050 / VL53L0X / HC-SR04 / Servo / L298N |
| `minimal` | Minimal (BME280 + LCD) | BME280 / LCD1602 |

### Device Toggle

各デバイスのチェックボックスで個別に有効 / 無効を切り替えられます。チェックを外したデバイスのパネルは `--` 表示になり、Wiring Diagram からも除外されます。

> **注意**: Arduino Nano は ESP32-CAM をサポートしないため、`full` プロファイルを選択してもカメラ項目は表示されません。

### Bus Labels (Show Bus Labels)

チェックすると Wiring Diagram にデバイス側の詳細ピンラベル (SDA/SCL/VCC/GND) が表示されます。実機配線の参考用です。

---

## Wiring Diagram

Wiring Diagram は `/api/wiring/svg` から取得した SVG をインラインで表示します。

- **共有バストランク**: 有効なデバイスが複数ある場合、I2C バス (SDA/SCL) と電源 (VCC/GND) は共有トランクに束ねられます。
- **デバイスブランチ**: 各デバイスはトランクから分岐して接続されます。
- **Show Bus Labels 無効時**: デバイス側のラベルは省略され、ボード側ピン名のみ表示されます。
- **Show Bus Labels 有効時**: デバイス側のピンラベルも表示されます。

---

## REST API

サーバーが `http://127.0.0.1:7878` で起動している前提です。

### GET /api/state

シミュレーター状態 (センサー値・アクチュエーター値) を JSON で返します。

```bash
curl http://127.0.0.1:7878/api/state | python3 -m json.tool
```

主なフィールド:

```json
{
  "board_name": "original ESP32",
  "mcu_name": "ESP32",
  "tick": 42,
  "i2c": { "operation_count": 100 },
  "climate": { "temperature_c": 25.1, "humidity_percent": 60.0, "pressure_pa": 101300.0 },
  "distance": { "distance_mm": 250 },
  "imu": { "accel_mg": [0, 0, 1000], "gyro_mdps": [0, 0, 0] },
  "servo": { "angle_degrees": 90 },
  "motor_driver": { "left": { "direction": "forward", "duty_percent": 50 }, "right": { ... } },
  "wiring": { "board": "esp32", "sensor_profile": "full", "selected_devices": [...], "show_bus_labels": false }
}
```

### GET /api/wiring

現在の配線設定 (board / profile / selected_devices / show_bus_labels) を返します。

```bash
curl http://127.0.0.1:7878/api/wiring | python3 -m json.tool
```

### POST /api/wiring

配線設定を更新します。送信したフィールドのみ更新されます。

```bash
# プロファイルを Climate Station に変更
curl -X POST http://127.0.0.1:7878/api/wiring \
  -H "Content-Type: application/json" \
  -d '{"sensor_profile": "climate"}'

# Arduino Nano + Minimal に変更
curl -X POST http://127.0.0.1:7878/api/wiring \
  -H "Content-Type: application/json" \
  -d '{"board": "nano", "sensor_profile": "minimal"}'

# Bus Labels を有効化
curl -X POST http://127.0.0.1:7878/api/wiring \
  -H "Content-Type: application/json" \
  -d '{"show_bus_labels": true}'

# デバイス個別指定
curl -X POST http://127.0.0.1:7878/api/wiring \
  -H "Content-Type: application/json" \
  -d '{"selected_devices": ["bme280", "lcd1602", "mpu6050"]}'
```

### GET /api/wiring/svg

現在の配線設定に対応した SVG を返します。

```bash
# SVG を保存
curl http://127.0.0.1:7878/api/wiring/svg -o wiring.svg
```

### GET /api/wiring/profiles

利用可能な sensor profile の一覧 (slug / display_name のペア) を返します。

```bash
curl http://127.0.0.1:7878/api/wiring/profiles | python3 -m json.tool
```

### GET /api/events

SSE (Server-Sent Events) エンドポイント。ブラウザの `EventSource` で自動接続されます。`data: <JSON>` 形式でシミュレーター状態を push します。

---

## Simulator → Dashboard → 実機 (sim-to-real)

### ステップ 1: PC Simulator で確認

```bash
# ESP32 / Full profile で起動
cargo run -p platform-pc-sim --bin device-dashboard-web

# ブラウザで http://127.0.0.1:7878 を開く
```

- 使用するセンサー・アクチュエーターを Device Toggle で選択
- Wiring Diagram で配線を確認 (Show Bus Labels を有効にするとピン名が表示される)
- SSE でセンサー値が更新されることを確認

### ステップ 2: 実機配線

Wiring Diagram で表示されるピン番号に従って配線します。

**ESP32 デフォルト配線 (I2C)**:

| ピン | 役割 |
|------|------|
| GPIO21 | SDA |
| GPIO22 | SCL |
| 3.3V | VCC (I2C センサー) |
| GND | GND |

**Raspberry Pi Pico デフォルト配線 (I2C)**:

| ピン | 役割 |
|------|------|
| GPIO4 | SDA |
| GPIO5 | SCL |
| 3.3V | VCC |
| GND | GND |

**BME280 デフォルト I2C アドレス**: `0x77`（SDO を 3.3V へ接続）
**LCD1602 バックパック**: `0x27`

### ステップ 3: Firmware ビルドと書き込み

```bash
# ESP32 climate display (espflash で書き込み)
cd firmware/original-esp32-climate-display
cargo run --release

# Raspberry Pi Pico climate display (probe-rs で書き込み)
cd firmware/raspi-pico-climate-display
cargo run --release
```

詳細は各 firmware ディレクトリの `README.md` を参照してください。

---

## ブラウザ対応状況

| ブラウザ | 対応状況 |
|---------|---------|
| Chrome / Chromium (最新) | ✅ 完全対応 |
| Firefox (最新) | ✅ 動作確認済み |
| Safari / WebKit (最新) | ✅ 動作確認済み |

SSE (`EventSource`) は全対象ブラウザでネイティブサポートされています。

> **CI**: `npm run test:e2e` で Chromium / Firefox / WebKit のクロスブラウザスモークテストを実行できます。

---

## トラブルシューティング

### ダッシュボードが「connecting…」のまま

- サーバーが起動しているか確認: `curl http://127.0.0.1:7878/api/state`
- ポートが競合していないか確認: `lsof -i :7878`
- 別ポートで起動: `cargo run -p platform-pc-sim --bin device-dashboard-web -- esp32 9000`

### センサー値が `--` のまま

- 該当デバイスが Device Toggle でチェックされているか確認
- SSE 接続状態 (`Online · updated Xs ago`) を確認

### Wiring Diagram が表示されない

- ブラウザの開発者ツールでネットワークエラーを確認
- `/api/wiring/svg` を直接 curl して SVG が返ってくるか確認

### プロファイル変更後に Wiring Diagram が更新されない

- ページをリロードして SSE を再接続
- DevTools の Network タブで `/api/wiring` POST が成功しているか確認

### Bus Labels が切り替わらない

- Show Bus Labels チェックボックスの状態を確認
- `/api/wiring` を curl して `show_bus_labels` フィールドを確認

---

## 関連ドキュメント

- [センサー・アクチュエーター一覧](./sensors-and-actuators.md)
- [ポーティング・拡張ガイド](./porting-and-extension-guide.md)
- [firmware/original-esp32-climate-display/README.md](../firmware/original-esp32-climate-display/README.md)
- [firmware/raspi-pico-climate-display/README.md](../firmware/raspi-pico-climate-display/README.md)
