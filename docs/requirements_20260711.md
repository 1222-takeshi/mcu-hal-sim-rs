# ESP32 OTA リクエスト検証ホストテスト 要件定義

## 目的

- ESP-IDF OTA 受信ファームの HTTP 入力検証を実機から分離し、ホスト上で回帰テストできるようにする。
- `POST /ota` の有効なクライアント I/F と HTTP ステータス契約を維持しながら、認証・長さ・ヘッダー境界の不具合を CI で検出する。

## スコープ

- 対象: request line、`X-OTA-Token`、`Content-Length`、payload 上限、ヘッダー 1 行・総量上限、UTF-8・EOF 異常。
- 対象外: ESP32 実機検証、TLS、rollback、OTA flash writer の mock 化、送信スクリプトのテスト。
- シークレットは API 引数へ渡し、ソース・テスト・ログへ実値を保存しない。

## I/F

| 項目 | 内容 |
|------|------|
| 入力 | `BufRead`、期待する OTA token、`RequestLimits` |
| 成功 | firmware body の `Content-Length` |
| 失敗 | `BadRequest` / `Unauthorized` / `HeaderTooLarge` / `LengthRequired` / `PayloadTooLarge` / `Io` |
| HTTP 契約 | 既存の `400` / `401` / `411` / `413` / `431` 対応を維持 |

## モジュール設計

- `firmware/original-esp32-ota-bringup/ota-http`: ESP-IDF に依存しない独立 workspace の parser crate。解析、制限値、エラー型、unit test を所有する。
- `firmware/original-esp32-ota-bringup`: parser crate を path dependency として利用し、TCP、ESP-IDF OTA writer、HTTP response を担当する。
- CI はリポジトリルートから `cargo test --manifest-path firmware/original-esp32-ota-bringup/ota-http/Cargo.toml` を実行し、ESP toolchain とビルド時シークレットを要求しない。
- ESP-IDF release build は実値ではない `OTA_WIFI_SSID=dummy`、`OTA_WIFI_PSK=dummy`、`OTA_AUTH_TOKEN=dummy` を指定して別経路で検証する。

## 入力検証方針

- request line は `POST /ota HTTP/1.0` または `POST /ota HTTP/1.1` の 3 要素だけを受理する。
- header 名は ASCII 大小文字を区別しない。OTA token の値は完全一致とする。
- `Content-Length` は 1 以上かつ OTA slot 上限以下とする。
- 同じ認証・長さ header の重複や colon のない header は曖昧性を避けるため `BadRequest` とする。
- header は 1 行 512 byte、request line を含む総量 2048 byte を上限とする。
- request line の余分な要素、`HTTP/1.0`・`HTTP/1.1` 以外、重複した認証・長さ header、colon のない header は、従来受理・無視していた場合も `400 Bad Request` とする。これは曖昧な解釈を排除する意図的な入力強化であり、既存の `curl` 送信経路には影響しない。

## テスト設計

| 観点 | ケース | 期待結果 |
|------|--------|----------|
| 正常 | HTTP/1.0・1.1、header 名の大小文字違い | `Content-Length` を返す |
| 認証 | token 欠落・不一致 | `Unauthorized` |
| 異常 | method、path、version、余分な request-line 要素 | `BadRequest` |
| 異常 | `Content-Length` 欠落・非数値 | `LengthRequired` |
| 境界 | length 0、上限、上限 + 1 | 0 と超過は `PayloadTooLarge`、上限は成功 |
| 境界 | header 1 行・総量の上限超過 | `HeaderTooLarge` |
| 異常 | 不正 UTF-8、header 終端前 EOF、重複 header | `BadRequest` |
| 異常 | reader の I/O error | `Io` として保持し、ファーム側で `400` に対応 |
| セキュリティ | token をテスト用固定値だけで検証 | 実シークレットを要求・出力しない |
| 可観測性 | error の表示文 | 機密値を含めず原因分類を示す |

## 受け入れ条件

- parser unit test が RED から GREEN になり、line coverage 80% 以上を満たす。
- ESP-IDF ファームが parser crate の実装を直接使用し、旧 parser の重複を残さない。
- parser test、repository test、format、lint、ESP-IDF release build が成功する。
- 実機を使わずに検証が完結し、有効な既存クライアントと HTTP response code 契約を変更しない。

## ロールバック

- parser crate、ファーム側の path dependency、CI step を同一 PR で戻す。
- OTA partition、rollback 設定、送信スクリプトには変更を加えないため、既存 PR #222 の実装へ差分単位で戻せる。

## 検証結果

- parser unit test: 11 件成功。
- parser Clippy: warning なし。
- parser line coverage: 91.87%。
- ESP-IDF release build: dummy の WiFi・token 値を使用して成功。
- repository test: 460 件成功、2 件 ignored。
- root / firmware / parser formatting: 成功。
- root Clippy: warning なし。
- 実機検証: 本 Issue の対象外として未実施。
