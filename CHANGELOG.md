# CHANGELOG

すべての重要な変更はこのファイルに記録されます。

このプロジェクトは [Semantic Versioning](https://semver.org/spec/v2.0.0.html) に準拠しています。

## [Unreleased]

## [2.7.1] - 2026-08-13

### 概要
`raw.githubusercontent.com` の CDN キャッシュにより、`latest.json` と `kasugai.exe` の内容が食い違って updater の署名検証エラーが繰り返し発生していた問題を修正しました。

### 修正 (Fixed)
- `run.py` が生成する `latest.json` のインストーラー URL に `?v={version}` クエリパラメータを付与。CDN はクエリを含む URL 全体でキャッシュするため、新バージョンの URL は必ず GitHub から最新ファイルを取得し、署名検証エラーを防止（キャッシュバスティング）。ファイル名は固定の `kasugai.exe` のまま。

### 変更 (Changed)
- プロジェクトバージョンを `2.7.0` から `2.7.1` に更新。

## [2.7.0] - 2026-08-13

### 概要
画面1に緊急時用の `🚨 リセット` ボタンを追加し、本番アプリから最新インストーラーを直接ダウンロード・起動できるようにしました。配布用インストーラー生成を `run.py` に統合し、Windows SDK `rc.exe` 自動検出も含むビルド自動化を完成させました。

### 追加 (Added)
- `index1.html` に `🚨 リセット` ボタンを追加。本番アプリから最新インストーラー（`download/kasugai.exe`）をダウンロードし、インストーラーを直接起動できる `reinstall_kasugai` Rust コマンドを実装。緊急時の再インストールを支援します。
- `run.py` に Windows SDK `rc.exe` 自動検出を追加し、`npx tauri build` で配布用 NSIS インストーラー、`kasugai.exe.zip`、`latest.json` を自動生成。
- 既存配布ファイルを `download/backup/` にバックアップする運用を追加。

### 変更 (Changed)
- プロジェクトバージョンを `2.6.9` から `2.7.0` に更新。

## [2.6.9] - 2026-08-13

### 概要
Yahoo Map との Cesium 双方向同期を見直し、緯度を考慮した高さ・zoom 換算式を導入しました。Cesium からの URL 取得を `history.replaceState` に変更し、自動同期の失敗を低減。OpenTopodata / Open-Elevation 取得を Rust 側で行うことで CORS 制限を回避。Google Earth / Google Maps 間の LookAt ターゲット点計算共通化も今回のリリースに含まれます。

### 追加 (Added)
- Rust 側に `get_terrain_elevation(lat, lng)` コマンドを追加。OpenTopodata / Open-Elevation を代理取得し、WebView の CORS 制限を回避。
- `index1.html` に `🚨 リセット` ボタンを追加。本番アプリから最新インストーラ（`download/kasugai.exe`）をダウンロードし、インストーラーを直接起動できる `reinstall_kasugai` Rust コマンドを実装。緊急時の再インストールを支援します。
- `run.py` に Windows SDK `rc.exe` 自動検出を追加し、`npx tauri build` を実行して配布用 NSIS インストーラー、`kasugai.exe.zip`、`latest.json` を自動生成。
- `index1.html` に `YAHOO_ZOOM_BASE` および `YAHOO_ZOOM_REF_LAT` 定数を追加。Yahoo Map の zoom ↔ `hAboveTarget` 換算に緯度補正を可能に。
- Cesium `get_pane2_url` 出力に `canvasZoom` / `viewportW` / `viewportH` / `computeViewRectangle` の可視範囲を追加（取得補助情報として保持）。

### 変更 (Changed)
- `index1.html` の Yahoo Map 移動を `calculateLookAtTarget()` 由来の `hAboveTarget` を使う方式に変更。`YAHOO_ZOOM_BASE + log2(cos(lat)/cos(refLat)) - log2(hAboveTarget)` で整数 zoom を算出。
- `index1.html` の Yahoo Map 取得時、ジオイド高・地形標高を加味して Cesium 用 `height` を逆算するように変更。
- `main.rs` の Cesium URL 取得を `window.location.replace` から `window.history.replaceState` に変更。URL 取得失敗を防止。
- `index2.html` の `switchTab` で `settings` / `default` など WebView を持たないタブからの切り替え時は `get_pane2_url` を呼ばないように変更。
- `get_pane2_url` の `canvasZoom` 計算を `pickEllipsoid` → `height/FOV` 近似 → `24.965 - log2(height)` の三段階 fallback に拡張。
- `index1.html` に `calculateLookAtTarget()` を追加し、Cesium カメラから地表 LookAt ターゲット点を計算する処理を Google Earth / Google Maps / Yahoo Map で共有。
- `moveMap()` の Google Earth / Google Maps 移動を `calculateLookAtTarget()` を使うよう簡潔化。`m` 値は `hAboveTarget` から算出。
- `getTerrainElevation()` にキャッシュを追加し、同一地点への重複標高 API 呼び出しを削減。

### 修正 (Fixed)
- `get_pane2_url` が `settings` など非 WebView タブで失敗して `自動同期: 切り替え前URL取得失敗` となる問題を軽減。
- Yahoo Map 自動同期で `canvasZoom` 未取得時に `URLを生成できませんでした` となる問題を修正（`hAboveTarget` 方式に切り替え）。
- `main.rs` の Cesium URL 取得処理を軽量化。`computeViewRectangle` / `pickEllipsoid` / `canvasZoom` 計算を削除し、タブ切替がブロックされる問題を修正。
- `index2.html` の自動同期で `get_active_pane2` / `get_pane2_url` に JS 側タイムアウトを追加。取得失敗時でもタブ切替を継続するように。
- `main.rs` の `get_geoid_undulation` に `#[allow(dead_code)]` を付与し、コンパイル時の未使用警告を抑制。

## [2.6.8] - 2026-08-12

### 概要
Google Earth との双方向変換にジオイド補正を導入し、Cesium（WGS84 楕円体高）と Google Earth（MSL）間の高さ変換を正確化しました。

### 追加 (Added)
- Rust 側に `egm2008` クレートを追加し、`get_geoid_undulation(lat, lon)` コマンドを実装。EGM2008 モデルでジオイド高を取得可能に。
- `index1.html` に `getGeoidUndulation()` ヘルパーを追加し、Rust コマンドを非同期で呼び出せるように。

### 変更 (Changed)
- `parseLocation()` を非同期化。Google Earth URL 取得時にカメラ位置のジオイド高を取得し、楕円体高（Cesium 基準）へ変換した `height` を返すように。
- `moveMap()` の Google Earth 移動で、カメラ直下とターゲット地点のジオイド高を考慮。`height - geoid - terrain` から真の地上高を算出し、`distance`/`target`/`a` を決定するように。

### 修正 (Fixed)
- `parseLocation()` の Google Earth 解析で、カメラ標高 `H` を `result.height` に追加。`getLocation()` が正確なカメラ高さを `input-height` に保持するよう修正。
- `moveMap()` の Google Earth 移動で、`H = 2^(25.2 - zoom)` というズーム換算近似を廃止し、入力された実カメラ高さを直接使用。
- Google Earth 移動 URL の `heading` を `normalizeBearing()` (0°〜360°) に統一。

## [2.6.7] - 2026-08-11

### 概要
ピッチを Cesium `camera.pitch` ネイティブに統一し、入力欄・移動処理・取得処理を見直しました。

### 変更 (Changed)
- **ピッチの Cesium ネイティブ化**: 画面1の入力欄と各パーサーの `pitch` を Cesium `camera.pitch` 方式（0°=水平、真下 -90°、下向きを負）に統一。
- **方位入力を Heading に変更**: 入力欄 `Bearing` を `Heading` に変更し、URL パラメータ・内部プロパティも `heading` に統一。
- **Re:Earth 取得を非対応化**: 画面2の取得ボタンで Re:Earth タブがアクティブな場合は「Re:Earth からの位置情報取得は非対応です。」と表示し、取得を行わないようにしました。
- **Google Maps 3D 取得対応**: `@lat,lng,alta,...,tiltt,headingh` 形式の Google Maps 3D/Earth view URL から tilt/heading を解析し、Cesium ネイティブの pitch/heading として反映。
- **ドキュメント更新**: `kasugai.md` / `re_erath_connect.md` の pitch 表記を Cesium ネイティブに合わせて更新。

### 修正 (Fixed)
- `get_pane2_url` での Cesium 取得時、pitch の符号反転を廃止し、Cesium `camera.pitch` をそのまま URL に反映。
- Re:Earth への移動 URL の pitch から符号反転を削除。

## [2.6.4] - 2026-08-10

## [2.6.4] - 2026-08-10

### 概要
CANVAS の URL 座標付加仕様の変更（クエリ `?` → ハッシュ `#`）に対応しました。

### 変更 (Changed)
- **CANVAS URL 形式の変更対応**: CANVAS への移動 URL を `http://127.0.0.1:8510/#latitude=...&longitude=...&zoom=...&pitch=...&bearing=...` のハッシュ形式で生成するように変更（従来はクエリ `?latitude=...` 形式）。
- **CANVAS 取得時の URL 更新**: `get_pane2_url` での `history.replaceState` もハッシュ形式で付加するように変更。旧形式のクエリが URL に残らないよう `location.pathname` を基準に置き換え。
- **解析の後方互換**: 位置情報のパースは `?` / `#` のどちらの形式も引き続き解析可能。

## [2.6.3] - 2026-08-10

### 概要
画面2のタブ表示設定を拡張し、プラットフォーム設定から GIS グループ・クラウドストレージ グループのタブを ON/OFF できるようにしました。

### 追加 (Added)
- **タブ表示 ON/OFF 設定**: プラットフォーム設定内に GIS グループとクラウドストレージ グループのタブを表示/非表示できるチェックボックスを追加。
- **GIS グループのタブ化**: QGIS / Google Map / Google Earth / Yahoo Map / Re:Earth の表示設定を GIS グループとしてサブタブ化。
- **クラウドストレージ グループのタブ化**: BOX / BOX_APP の表示設定をクラウドストレージ グループとしてサブタブ化。
- **設定の永続化**: タブの表示/非表示を `localStorage` および INI ファイル入出力に対応。

### 変更 (Changed)
- **プラットフォーム設定の UI 整理**: GIS グループとクラウドストレージ グループをサブタブで切り替え表示。

## [2.6.2] - 2026-08-09

### 概要
バージョンを 2.6.2 に更新し、配布用インストーラーと更新情報を再作成しました。

## [2.6.1] - 2026-08-09

### 概要
画面1の GIS 連携 UI を「自動同期」ボタンで切り替え可能なトグル形式に縮小化し、GIS タブ切り替え時に移動前の位置を自動取得・移動後に反映する自動同期機能を追加しました。Yahoo Map / Google Map からの取得時は平面表示として pitch=90°, bearing=0° を固定します。

### 追加 (Added)
- **自動同期トグル UI**: 画面1の緯度/経度/縮尺/傾斜/方位入力欄をクリックすると「自動同期」ボタンを表示。自動同期モードでは「取得」「移動」ボタンを非表示にして UI を縮小化。
- **タブ切り替え時の自動同期**: 自動同期モード中に画面2の GIS タブを切り替えると、切り替え前のタブ URL から座標を取得し、切り替え後のタブに移動を適用。自動同期対象は **CANVAS / Google Maps / Google Earth / Yahoo Map** のみ。
- **Google Earth URL 待機取得**: Google Earth の URL に座標（`@...`）が含まれるまで最大 15 秒待機してから自動同期を実行。
- **対象外タブの通常切り替え**: CANVAS / Google Maps / Google Earth / Yahoo Map 以外のタブでは緯度経度を反映せず、`index2` 側の通常タブ切り替え処理（移動先タブの持つ URL・データ）で移動する。
- **WebView 切り替え後の URL オープン**: 自動同期時に `switch_pane2_tab` で切り替え先 WebView を表示してから `open_in_pane2` で URL を開くよう順序を制御。これにより Google Earth への自動同期も安定して動作。

### 変更 (Changed)
- **Yahoo Map / Google Map の pitch/bearing 固定**: Yahoo Map / Google Map からの位置取得時、平面表示を前提として常に `pitch=90.00`、`bearing=0.00` とするよう各パース経路を統一。

## [2.6.0] - 2026-08-09

### 概要
CANVAS（CesiumJS）を基準に、Google Maps / Yahoo 地図 / Google Earth 間の位置同期・移動ロジックを統一・整理しました。標高を考慮した中心位置計算、Google Maps `m` 値との双方向換算、および仕様書の更新を行いました。

### 追加 (Added)
- **CANVAS 基準の位置同期統一**: 全地図サービスの取得・移動を CANVAS のカメラ（オービット）位置を基準に正規化。
- **Google Maps `m` 値 ↔ CANVAS `zoom` 換算**: 実測データに基づき、`zoom = 23.663 - 0.9561 * log2(m)`、`m = 2 ^ ((23.663 - zoom) / 0.9561)` を導入。
- **標高考慮の中心位置計算**: CANVAS カメラ高度から `0.83` 倍補正とターゲット点の標高を差し引いた有効高度 `H_effective = max(0, 0.83 * 2^(25.2 - zoom) - terrainAlt)` で `ground = H_effective / tan(pitch)` を算出。
- **Google Maps 2D 傾斜表示範囲**: `pitch` に応じて `m` 値を `1/sin(pitch)` 倍（最大20倍）で広げる。
- **Yahoo 地図への CANVAS 基準ロジック適用**: Google Maps 2D と同じく標高・0.83補正を Yahoo 地図 2D 移動にも適用。

### 変更 (Changed)
- **仕様書更新**: `kasugai.md` の位置同期セクションを CANVAS 基準の最新ロジックに更新。

## [2.5.6] - 2026-08-08

### 概要
画面2の CesiumJS タブを CANVAS (`http://127.0.0.1:8510/`) に変更し、起動時の初期表示・最左配置としました。CANVAS の位置情報形式に対応した取得・移動も可能になっています。

### 追加 (Added)
- **CANVAS 連携**: 画面2の地図タブを `http://127.0.0.1:8510/` で動作する CANVAS に変更しました。
- **Google Maps 2D 移動対応**: CANVAS のカメラ位置・pitch・bearing から Google Maps 2D の表示中心 `@lat,lng` とズームレベルを、低ズームにも対応した Google Maps 用カメラ高度 `H = 2^(25.54 - 1.075 * zoom)` で算出。表示中心までの距離を `s = H / tan(pitch)`、ズームを `z = max(7.75, zoom + 1.0 + 0.6 * (pitch/90))` として求めるようにしました。
- **Yahoo 地図 2D 移動対応**: 提供された 4 ペアに基づき、Yahoo 用カメラ高度 `log2(H_y) = -0.0687*zoom^2 + 0.4568*zoom + 18.0562`、中心距離 `s = H_y / tan(pitch)`、ズーム `z_yahoo = max(8, round(zoom + 0.2))` で `?lat=&lon=&zoom=` を算出するようにしました。
- **CANVAS 位置情報の取得・移動**: `?latitude=...&longitude=...&zoom=...&pitch=...&bearing=...` 形式の URL を、画面1の「取得」「移動」で解析・生成できるようにしました。
- **画面1 Pitch/Bearing 入力**: 画面1に `Pitch`・`Bearing` 入力欄を追加し、CANVAS の位置情報取得・移動で反映。2D 地図からの取得は真上（Pitch=0.00、Bearing=0.00）を前提とします。
- **Google Earth 3D 取得・移動対応**: Google Earth URL 内の `t`（tilt / 上方向からの角度）・`h`（bearing / heading）・`d` 値を取得し、画面1の入力欄に反映。Google Earth の `@lat,lng` は LookAt ターゲットであるため、取得時に `t`/`h`/`d` からカメラ位置を逆算して CANVAS 形式に返すようにしました。CANVAS の `?pitch=` は水平線からの下向き角なので、Google Earth では `t = 90 - pitch`、カメラ高度を `H = 2^(25.2 - zoom)` に換算し、ターゲットまでの斜距離 `d = H / cos(t)`、地面投影距離 `d * sin(t)` を使って LookAt ターゲットを haversine 計算で求めるようにしました。

### 変更 (Changed)
- **タブラベル**: `CesiumJS` から `CANVAS` に表示名を変更しました。
- **タブ配置・初期表示**: CANVAS タブをタブバー左端に配置し、起動時の初期表示タブとしました。

### 修正 (Fixed)
- **CANVAS 取得の URL 更新**: CANVAS 側が URL クエリを更新しないため、`get_pane2_url` 実行時に Cesium カメラ (`window.viewer.camera`) から現在の緯度・経度・高度・pitch・heading を算出し、URL を `history.replaceState` で更新してから取得するようにしました。これにより、CANVAS 内で手動移動した後も KASUGAI の「取得」で現在の値が取得できるようになりました。

### 削除 (Removed)
- **不要な cesium.html の削除**: CesiumJS タブを CANVAS に変更したため、使用しなくなった `kasugai/src/cesium.html` を削除しました。

## [2.4.0] - 2026-07-28

### 概要
画面2に KASUGAI_BOX サイドカー専用の `BOX_APP` タブを追加し、KASUGAI 本体からのインストール・起動・状態監視を統合しました。合わせて、画面2タブ全体の再読み込みボタンを追加し、Google Map 専用の更新ボタンを削除しました。

### 追加 (Added)
- **BOX_APP タブ**: `http://127.0.0.1:8410/ui` を専用 WebView（`pane2_boxapp`）で表示する `BOX_APP` タブを追加しました。
- **KASUGAI_BOX サイドカー統合**: 画面2の「システム設定 > BOX」タブに KASUGAI_BOX のインストール先、インストール/起動ボタン、状態表示を追加しました。
- **KASUGAI_BOX 状態監視**: `http://127.0.0.1:8410/health` への応答で実際のサービス起動状態を判定し、起動/停止を正しく表示するようにしました。
- **汎用タブ再読み込みボタン**: タブバー先頭に 🔄 ボタンを配置し、現在アクティブな画面2タブを再読み込みできる `reload_pane2` コマンドを追加しました。

### 変更 (Changed)
- **Google Map 専用更新ボタンの削除**: タブバーにあった `🔄`（Google Map 再読み込み）ボタンを削除し、全タブ共通の更新ボタンに一本化しました。

## [2.3.1] - 2026-07-25

### 概要
画面1の GIS URL 入力欄の下にシステム設定を開く固定ボタンを追加し、画面2のタブバーから「⚙️ システム設定」タブを削除しました。GIS URL 入力欄クリック時のクリップボード自動貼り付けは、URL のみを貼り付けるように制限しました。

### 追加 (Added)
- **画面4のダブルクリック最大化**: 画面4（中央下ペイン）に専用の初期化スクリプト（`pane: 'pane4'`）を追加し、面のダブルクリックで最大化・復元できるようにしました（従来は誤って画面2として扱われていた不具合を修正）。
- **スプリッターダブルクリックによる表示・非表示トグル**: スプリッター2 ➔ 画面3、水平スプリッター ➔ 画面4 の表示・非表示をトグルする Rust コマンド `toggle_pane3` / `toggle_pane4` を追加。非表示前の幅・高さを保存し、再表示時に復元します（スプリッター1 ➔ 画面1 は従来どおり）。

### 変更 (Changed)
- **画面2・3・4の最大化仕様**: 最大化は「画面2・3・4のうち1つを大きくする」動作に変更。画面1の表示状態（開閉）には触れず、復元時も `ratio2` / `pane4_ratio` のみを戻すようにしました。
- **システム設定タブの配置**: 画面2のタブバーにあった「⚙️ システム設定」タブを削除し、画面1の GIS URL 入力欄の下に固定ボタン「⚙️ 設定」として移動。`pane2_open_settings` イベントで画面2のシステム設定を開くようにしました。
- **境界（スプリッター）の常時表示**: 最大化・非表示時でもスプリッター2（左右端でクランプ）と水平スプリッター（画面4最大化時もタブバー50px＋8pxを確保）が常に画面内に残るようレイアウト計算を調整しました。

### 修正 (Fixed)
- **ダブルクリック時の誤ドラッグによる比率破壊**: ダブルクリックに伴う微小な `mousemove` / `pointermove` でスプリッター比率が上書きされ、画面3の非表示前の幅が復元されない問題を修正。全スプリッターに 4px の移動しきい値を導入しました。
- **GIS URL 入力欄のクリップボード誤貼り付け防止**: 入力欄クリック時の自動貼り付けで、クリップボード内の無関係なテキスト（「プラットフォーム設定」など）が入力されてしまう問題を修正。`http://` / `https://` で始まる URL のみを貼り付けるようにしました。

### 変更
- **Windows NSIS インストーラー**: `installMode` を `perMachine` から `currentUser` に変更。UAC なしでインストールできるようにしました。
- **WebView2 インストールのスキップ**: `webviewInstallMode` を `skip` に設定。Windows インストーラー内で WebView2 ランタイムのインストールを行わないようにしました。
- **デフォルトインストール先**: `C:\kasugai` をデフォルトとし、ディレクトリページでユーザーが自由に変更可能にしました。

## [2.3.0] - 2026-07-24

### 概要
QGIS Launcher (`kasugai_qgis`) を KASUGAI からインストール・起動できるようにしました。インストール先は画面2のシステム設定「QGISランチャー」タブで変更可能です。QGIS ランチャー本体のバージョンアップ等は、今後 `qgis_launcher` 側で実施する前提とし、KASUGAI 側はあくまでインストール・起動のみを担当します。

### 追加 (Added)
- **QGIS Launcher 統合**: 画面2のシステム設定に「QGISランチャー」タブを追加。`kasugai_qgis` の NSIS インストーラー (`kasugai_qgis-setup.exe`) をダウンロードし、指定フォルダにサイレントインストール (`/S /D=<インストール先>`) する機能を追加しました。
- **インストール先の指定**: デフォルトは `C:\Kasugai\kasugai_qgis`。ユーザーが入力欄でインストール先を自由に変更できます。
- **INI 保存/読み込み対応**: `qgis_launcher_install_path` キーを KASUGAI の INI ファイル保存・読み込みに追加しました。
- **起動ボタン**: 未インストール時は「インストール」、インストール済み時は「起動」に切り替わるボタンを提供します。

### 変更 (Changed)
- **デフォルトインストール先**: QGIS Launcher の初期インストール先を `%LOCALAPPDATA%\Kasugai\qgis_launcher` から `C:\Kasugai\kasugai_qgis` に変更しました。

## [2.2.0] - 2026-07-24

### 概要
画面2（中央ペイン）のGoogle Map等の専用WebView表示と、画面4（下部ペイン）との水平スプリッター表示・操作を整理しました。専用画面がスプリッターを覆ってしまいドラッグできなかった問題を修正しています。

### 追加 (Added)
- **画面4水平スプリッターの可視化**: 専用WebView（Google Maps / Google Earth / Yahoo Map / CesiumJS / BOX / Re:Earth）の下端に8pxのスプリッター余白を確保し、常にドラッグ可能な水平スプリッターバー（`#hsplit`）を表示するようレイアウト計算を調整しました。

### 変更 (Changed)
- **レイアウト計算の調整**: `recalculate_webview_bounds` で `pane2_*` 専用WebViewの高さから水平スプリッター幅（8px）を差し引くよう変更しました。
- **`index2.html` の#hsplit CSS**: 縮小しないよう `flex-shrink: 0` を追加しました。

## [2.1.0] - 2026-07-20

### 概要
Re:Earth プラグインとの連携を強化しました。プラグインが生成する permalink を画面側で直接読み取り、画面1の「取得」操作で緯度・経度・ズームを自動抽出できるようにしました。また、画面1からの移動操作時に Re:Earth 用の permalink を生成してクリップボードへコピーする機能を追加しました（見下ろし表示: `heading=0&pitch=-90` を付与）。

### 追加 (Added)
- **Re:Earth permalink の読み取り**: 画面1 の取得ボタンが Re:Earth タブ選択時、クリップボードから permalink を読み取り、`Lat` / `Lng` / `Zoom` を自動入力する機能を追加しました。
- **Re:Earth への移動（コピー方式）**: 画面1 の移動操作で Re:Earth がアクティブな場合、`lat`/`lng`/`zoom` から Re:Earth 用の URL を組み立ててクリップボードへコピーする機能を追加しました（`heading=0&pitch=-90` を付与）。

### 変更 (Changed)
- 位置・ズーム抽出ロジックを強化し、Re:Earth の `height` パラメータからズームを近似算出するアルゴリズムを導入しました。

## [2.0.0] - 2026-07-15

### 概要
4画面構成（画面1/画面2/画面3/画面4）へ進化しました。中央ペイン（画面2）の下部に汎用HTML表示領域（画面4）を追加し、GIS と AI を同時表示・操作できる統合体験を強化しました。

### 追加 (Added)
- **画面4（汎用HTMLビューワ）**: `src/index4.html` を追加。外部HTMLの取得・直接HTML注入に対応し、スクリプトは実行しない安全な表示モードを提供します。
- **pane4 比率同期コマンド**: Rust 側に `update_pane4_ratio` コマンドを追加し、画面間でスプリッタ位置を同期可能にしました。

### 変更 (Changed)
- **スプリッター同期ロジックの改善**: 水平スプリッター（画面2/4間）に対するドラッグ挙動を改善。ドラッグ中は視覚的ゴーストで追従し、放した時点で比率を確定する挙動に統一しました。

### 互換性と注意点
- 既存の設定 (`localStorage` の `pane4_ratio`) が小さい値を持っている場合に初回起動で補正されます。


## [1.2.0] - 2026-07-13

### 概要
CesiumJSにおける位置同期・カメラ移動処理をURLハッシュ（`#lat=...&lon=...`）同期方式へと完全移行し、他の地図サービス（Google Maps等）と同一のパラメータによるシームレスな双方向移動・同期を可能にしました。また、初期起動時のデフォルト表示位置（東京駅）への自動ナビゲーション仕様を最適化・強化しました。

### 追加 (Added)
- **CesiumJS 位置同期のURL（ハッシュ）化統合**:
  - 位置同期（移動と取得）にURLハッシュパラメータを採用。他のペインやシステム設定からのシームレスな同期移動・同期取得に対応しました。
- **起動時の最速自動ハッシュ付与・東京駅初期表示処理**:
  - `cesium.html` の初期ロード時、ハッシュパラメータがない場合は即座に「東京駅」のデフォルト座標ハッシュをURLに動的追加する最速初期化コードを導入。
  - 地図読み込みの非同期エラーや接続遅延があっても確実にカメラ位置を東京駅にセットアップする堅牢な初期化フローを構築。

### 変更 (Changed)
- 仕様ドキュメント（`kasugai.md`）の更新：CesiumJSがURLハッシュ方式での座標同期移動に標準対応したことに伴う記述の全面的なブラッシュアップ。

## [1.1.0] - 2026-07-13

### 概要
画面2（中央ペイン）および画面3（右ペイン）の個別タブ・ウィンドウを独立したウィンドウとして切り離す「デタッチ機能」を新規実装しました。これにより、マルチモニター環境などでの視認性と操作性が飛躍的に向上しました。

### 追加 (Added)
- **デタッチ（独立ウィンドウ）機能の実装**：
  - **画面2（中央ペイン）**: 各種タブ（Cesium、Re:Earth、Google Earth等）の右クリックによるダブルクリック/特定操作で、独立したWebView2ウィンドウとしてデタッチ可能に。
  - **画面3（右ペイン）**: Gemini AIアシスタント画面や拡張タブのダブルクリック操作により、単独のウィンドウとして切り離し（デタッチ）可能に。
  - **相互通信と状態同期**: デタッチされたウィンドウが閉じられた際は、自動的にメインウィンドウの元のペインにタブ表示および機能が復元・同期される仕組みを構築。

## [1.0.0] - 2026-07-12

### 概要
Kasugaiプロジェクトの初版（v1.0.0）リリース。Tauri × Rustを採用した、自己進化型・空間統合DXプラットフォームのベースライン機能を確立しました。QGIS、クラウドストレージ（Box等）、AI（Gemini）などの異種システムをセキュアに繋ぐ、地図・空間データ特化型の次世代マルチウィンドウ・ブラウザシステムです。

### 追加 (Added)
- **マルチウィンドウ（3ペイン）の統合UIレイヤーの構築**：
  - **画面1（左ペイン）**: 常時最小幅80px固定、自動クローズ・復元、閉塞時のネオン発光エフェクトといった利便性と意匠性を両立した革新的UI。
  - **画面2（中央ペイン）**: 加工を一切行わない通常ブラウザ表示（WebView2）により、外部サービス（Google Earth、Google Maps、各種WebGIS等）の利用規約に100%準拠した状態で100%のオリジナル機能を描画。
  - **画面3（右ペイン）**: 外部リンクの自動引き込みが可能な動的タブブラウザ兼AIアシスタント画面。
- **ナビゲーションインターセプト＆自動ルーティング**：
  - 画面2（地図）内の外部リンクや新規ウィンドウ要求を検知・傍受し、画面3（右ペイン）へ強制ルーティング。これにより、AIとGISの常時表示を維持。
- **自己進化型AIアシスタント機能（画面3）**：
  - 多世代Geminiモデルの動的切り替え機能を実装（Flash系・Pro系モデル等を用途にあわせて選択可能）。
  - AI回答表示部におけるMarkdown（marked.js）によるリッチテキスト描画の導入。
  - Gemini APIのトークン使用量メーターUIの表示機能。
  - AI入力入力欄を2行に変更し、使いやすさを向上。
- **GIS連携の高速化・最適化**：
  - Google Earth上での移動処理の高速化、および位置取得・移動処理のノイズ（不要なテキストコメント）除去。
- **セキュア環境・LGWAN展開支援**：
  - Sidecar方式によるオフライン環境でのインストーラー自動展開・サイレントインストール機能。
- **プロジェクトドキュメント（GitHub Pages）の公開**：
  - `index.md`, `kasugai.md`, `readme.md` などの詳細仕様書の完備。

### 変更 (Changed)
- ドキュメント（KASUGAIシステム専用ブラウザ設計思想と規約準拠に関する記述）の構成・校正。
