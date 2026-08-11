これまでの検証結果を全てまとめます。

---

## Re:Earth 連携まとめ

### アーキテクチャ

```
【Re:Earth プラグイン構造】

WebAssembly側（QuickJS）
  ├── reearth API使用可
  ├── ブラウザAPI使用不可（location等）
  ├── 外部HTTP通信不可
  └── postMessageでiframe側と通信

iframe側（ブラウザ）
  ├── ブラウザAPI使用可
  ├── 外部HTTP通信可（CORS必要）
  ├── WebSocket通信可（CORS不要）
  ├── window.parent.location 読取不可（nullオリジン）
  └── postMessageでWebAssembly側と通信
```

---

### 外部通信手段

| 手段 | 可否 | 条件 | 双方向 | リアルタイム |
|---|---|---|---|---|
| **URL パラメータ（Visualizer 本体）** | ✅ | `https://*.visualizer.reearth.io/?lat=...&lng=...&height=...&heading=...&pitch=...` | - | 起動時／リロード時 |
| **WebSocket** | ✅ | wss://推奨 | ✅ | ✅ |
| **fetch / XHR** | ✅ | CORS必要 | 擬似 | △ |
| **SSE** | ✅ | CORS必要 | 受信のみ | ✅ |
| **URL パラメータ読取（プラグイン）** | ❌ | nullオリジン制約 | - | - |
| **location.href（プラグイン）** | ❌ | QuickJS制約 | - | - |
| **localStorage（プラグイン）** | ❌ | nullオリジン制約 | - | - |

---

### 今回のユースケース解決策

KASUGAI は Re:Earth Visualizer 本体に対して、URL クエリパラメータで直接カメラ位置を指定して開きます。

```
KASUGAI
    ↓ open_in_pane2({ url: reearthUrl })
Re:Earth Visualizer（WebView2）
    ↓ URL パラメータを読み取り
    ↓ カメラを指定座標に移動
地図が指定座標に移動 ✅
```

---

### URL 形式

```
https://<project-id>.visualizer.reearth.io/
  ?lat=<latitude>
  &lng=<longitude>
  &height=<height>
  &heading=<heading>
  &pitch=<pitch>
```

例：

```
?lat=35.188733&lng=138.610404&height=9807.7&heading=28.56&pitch=24.72
```

### パラメータの意味

すべて Cesium ネイティブの値を使用します。Cesium の `lookAt` / `lookAtTransform` では公式に **Positive pitch angles are below the plane** と定義されており、本連携ではこれを **Cesium ネイティブ** として採用します（0°=水平、真下 90°、下向きを正）。

| パラメータ | Cesium ソース | 説明 |
|---|---|---|
| `lat` | `positionCartographic.latitude` | 緯度（度） |
| `lng` | `positionCartographic.longitude` | 経度（度） |
| `height` | `positionCartographic.height` | カメラ高度（楕円体からの高さ、m） |
| `heading` | `camera.heading` | 方位角（度、0°=北） |
| `pitch` | `camera.pitch` の符号反転（`-camera.pitch`） | 傾斜（度、0°=水平、真下 90°、下向きを正） |

---

### 実装構成

**① KASUGAI 側（Rust / JavaScript）**

- `main.rs` の Cesium 用 WebView から `camera.positionCartographic` と `camera.heading` / `camera.pitch` を取得
- 取得した値を URL クエリに変換
- `open_in_pane2` または `open_reearth_in_pane` で WebView2 / Re:Earth Visualizer を開く

**② URL パラメータ送信例**

```js
const reearthUrl = `${base}?lat=${lat}&lng=${lng}&height=${height}&heading=${heading}&pitch=${pitch}`;
await window.__TAURI__.core.invoke('open_in_pane2', { url: reearthUrl });
```

**③ Re:Earth Visualizer 側**

- Visualizer 本体が URL パラメータを読み取り、起動時にカメラを移動
- 手動・自動同期の両方で同じ URL 生成ロジックを使用

---

### 制約まとめ

| 制約 | 理由 | 回避策 |
|---|---|---|
| プラグインが URL を読めない | QuickJS＋nullオリジン | Visualizer 本体の URL パラメータを使用 |
| プラグイン localStorage 不可 | nullオリジン | KASUGAI 側 `localStorage` / 設定ファイルで管理 |
| バイナリ送信不可 | postMessage制約 | base64エンコード（必要な場合） |
| CORS必要（fetch） | nullオリジン | WebSocket または URL パラメータで回避 |
| プラグインzip10MB上限 | Re:Earth仕様 | 画像等は外部URLで参照 |

---

### 推奨構成

```
KASUGAI → open_in_pane2 → Re:Earth Visualizer
```

URL パラメータ方式が、手動・自動同期を含めた現時点で最もシンプルかつ確実な連携手段です。
