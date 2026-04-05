# core-app

`core-app` は、`hal-api` 上に載る `no_std` の共通アプリロジック crate です。

この crate の役割は、board / MCU / host 環境の差分を知らずに、アプリケーションとしての振る舞いを固定することです。

## 含まれるもの

- `App`: GPIO / I2C を使う最小アプリ例
- `climate_display::ClimateDisplayApp`
  - 温湿度を 16x2 表示へ流す reference app
  - simulator と実機の両方で同じロジックを再利用可能

## 設計方針

- `std` に依存しない
- timing や observability の方針は config で切り替える
- board 固有の初期化や pin assignment は platform crate 側へ閉じ込める

## 拡張方針

- 新しい board を増やすときは `core-app` を変えず、まず `hal-api` と platform crate の責務分離で吸収する
- 新しい sensor を増やすときは、まず `EnvSensor` など既存 trait へ載るかを確認し、足りない契約だけを追加する
- `ESP32-CAM` のような board 固有性が強い案件は、まず別 firmware / platform で実験し、共通化できる契約だけを戻す
