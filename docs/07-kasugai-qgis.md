---
title: 7. KASUGAI_QGIS サイドカー
nav_order: 7
---

## 7. KASUGAI_QGIS サイドカー紹介

**KASUGAI_QGIS**（`kasugai_qgis` / `qgis_launcher`）は、QGIS / QField の起動を制御する KASUGAI 用サイドカーです。
GPL-3.0-only の QGIS 周辺処理を KASUGAI 本体から独立したプロセスとして分離することで、ライセンス影響を防ぎながら、Web UI / HTTP API / CLI から QGIS 操作を提供します。

![KASUGAI_QGIS](./images/kasugai_qgis.png)

### 主な機能

- QGIS / QField の Web UI / CLI 起動
- プロファイル・プロジェクト・QGIS バージョンの切り替え
- ユーザーロール制御（`Viewer` / `Editor` / `Administrator`）
- クラウドドライブ自動割り当て（`drive_mappings`）
- ローカル自動同期（`local_sync`）
- NSIS ベースの自動更新

### 技術構成

| 項目 | 内容 |
| :--- | :--- |
| 実装 | Rust + Axum |
| バージョン | `2.0.4` |
| ライセンス | GPL-3.0-only |
| 既定ポート | `8500`（`127.0.0.1` 固定） |
| 設定ファイル | `qgis_settings.json` |

### KASUGAI からの接続

`qgis_launcher.exe --server` を起動すると、`http://127.0.0.1:8500` で Web UI と HTTP API が利用できます。
KASUGAI からはペインまたはタブでこの URL を読み込み、QGIS 起動やプロファイル管理を行います。

主要なエンドポイント例：

| 用途 | メソッド・パス |
| :--- | :--- |
| Web UI | `GET /` |
| ヘルスチェック | `GET /health` |
| 設定取得・更新 | `GET/POST /settings` |
| QGIS 実行ファイル一覧 | `GET /qgis` |
| プロファイル一覧 | `GET /profiles` |
| プロジェクト一覧 | `GET /projects` |
| QGIS 起動 | `POST /launch` |
| プロファイル再配布 | `POST /reset` |
| 進捗確認 | `GET /progress` |
| プロジェクトファイルバージョン | `GET /project-version` |
| サーバー情報 | `GET /api/v1/server/info` |
| サーバー停止 | `POST /api/v1/server/stop` |

### 設定の読み込み順序

`qgis_settings.json` は以下の優先順で読み込まれます：

1. `qgis_settings.json`（ベース設定）
2. `qgis_settings_override.json`（全ユーザー強制上書き）
3. `qgis_settings_{USERNAME}.json`（ユーザー個別上書き）

③が最後に適用されるため、ユーザー個別設定が最も優先されます。

### ダウンロード・詳細

- 導入ドキュメント: <https://yamamoto-ryuzo.github.io/kasugai_qgis/>
- 初回インストーラー: <https://yamamoto-ryuzo.github.io/kasugai_qgis/public/kasugai_qgis-setup.exe>
- リポジトリ: <https://github.com/yamamoto-ryuzo/kasugai_qgis>
