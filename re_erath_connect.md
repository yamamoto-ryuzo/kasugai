これまでの検証結果を全てまとめます。

---

## Re:Earthプラグインの外部通信 完全まとめ

### アーキテクチャ

```
【Re:Earthプラグイン構造】

WebAssembly側（QuickJS）
  ├── reearth API使用可
  ├── ブラウザAPI使用不可（location等）
  ├── 外部HTTP通信不可
  └── postMessageでiframe側と通信

iframe側（ブラウザ）
  ├── ブラウザAPI使用可
  ├── 外部HTTP通信可（CORS必要）
  ├── WebSocket通信可（CORS不要）✅検証済み
  ├── window.parent.location 読取不可（nullオリジン）✅検証済み
  └── postMessageでWebAssembly側と通信
```

---

### 外部通信手段

| 手段 | 可否 | 条件 | 双方向 | リアルタイム |
|---|---|---|---|---|
| **WebSocket** | ✅ | wss://推奨 | ✅ | ✅ |
| **fetch / XHR** | ✅ | CORS必要 | 擬似 | △ |
| **SSE** | ✅ | CORS必要 | 受信のみ | ✅ |
| **URLパラメータ読取** | ❌ | nullオリジン制約 | - | - |
| **location.href** | ❌ | QuickJS制約 | - | - |
| **localStorage** | ❌ | nullオリジン制約 | - | - |

---

### 今回のユースケース解決策

```
KASUGAI（外部システム）
    ↓ URLから緯度経度を読み取り
    ↓ WebSocketサーバーに送信
        ↕ wss://
Re:Earthプラグイン（iframe側）
    ↓ WebSocketで受信
    ↓ postMessage
WebAssembly側
    ↓ reearth.camera.flyTo()
地図が指定座標に移動 ✅
```

---

### 実装構成

**① Re:Earthプラグイン（検証済み）**

```js
// ws-camera-widget.js
reearth.ui.show(`
<html>
<body>
  <script>
    const ws = new WebSocket("wss://your-server.com/ws");

    ws.onmessage = (e) => {
      const data = JSON.parse(e.data);
      if (data.type === "move_camera") {
        parent.postMessage(data, "*");
      }
    };
  <\/script>
</body>
</html>
`, { visible: false });

reearth.on("message", (msg) => {
  if (msg.type === "move_camera") {
    reearth.camera.flyTo(
      { lat: msg.lat, lng: msg.lng, altitude: msg.alt || 10000 },
      { duration: 2 }
    );
  }
});
```

**② WebSocketサーバー（外部）**
```json
// 送受信するメッセージ形式
{
  "type": "move_camera",
  "lat": 35.6849969,
  "lng": 139.7554625,
  "alt": 10000
}
```

**③ KASUGAI側**
```
URLから緯度経度を抽出
    ↓
WebSocketサーバーに上記JSON形式で送信
```

---

### 制約まとめ

| 制約 | 理由 | 回避策 |
|---|---|---|
| URLが読めない | QuickJS＋nullオリジン | KASUGAIが読んでWebSocketで送る |
| localStorage不可 | nullオリジン | WebSocketサーバーで管理 |
| バイナリ送信不可 | postMessage制約 | base64エンコード |
| CORS必要（fetch） | nullオリジン | WebSocket使用で回避 |
| プラグインzip10MB上限 | Re:Earth仕様 | 画像等は外部URLで参照 |

---

### 推奨構成

```
KASUGAI → WebSocketサーバー → Re:Earthプラグイン
```

これが現時点で**最もシンプルかつ確実**な外部通信手段です。次はWebSocketサーバーの構築に進みますか？