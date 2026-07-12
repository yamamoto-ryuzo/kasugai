// 1. Re:EarthのWidget UIとして、完全に不可視・透明（幅0、高さ0）のHTMLを描画させる
// （※Re:Earthは ui.show を使って中継iframeを作らせないとプラグイン自体がロードエラーになるため、幅・高さ0で配置します）
const widget_html = `
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <style>
        body { margin: 0; padding: 0; background: transparent; overflow: hidden; }
    </style>
</head>
<body>
    <script>
        console.log("[Kasugai Re:Earth Plugin] カメラ同期ウィジェット(widget)が起動しました。");

        // KASUGAI(Tauri)の最外層Webviewから送られてくる W3C標準 postMessage を待ち受ける
        window.addEventListener("message", (event) => {
            const data = event.data;
            if (data && data.action === "sync_camera") {
                const { lat, lon, height } = data.payload;
                console.log("[Kasugai Re:Earth Plugin] カメラ同期命令を検知しました:", lat, lon, height);

                // 隔離されたプラグインiframeから、Re:Earthコアプログラム（親Window）へメッセージを中継
                parent.postMessage({
                    action: "reearth_flyto",
                    camera: {
                        lat: lat,
                        lng: lon,
                        height: height,
                        heading: 0,
                        pitch: -1.5, // 真下を見下ろすピッチ角
                        roll: 0
                    }
                }, "*");
            }
        });
    </script>
</body>
</html>
`;

// Re:Earth公式のUI表示。画面を汚さないように「幅0, 高さ0」でマウントして起動させます。
reearth.ui.show(widget_html, { width: 0, height: 0, visible: true });

// 2. widget.html(iframe)から送られてきた中継メッセージを最外層で受け取り、カメラ同期APIを叩く
reearth.on("message", (msg) => {
    if (msg && msg.action === "reearth_flyto") {
        const { lat, lng, height, heading, pitch, roll } = msg.camera;
        
        // 最外層コンテキストで実行されている Re:Earth公式カメラ操作APIをダイレクトにコール！
        reearth.visualizer.camera.flyTo({
            lat: lat,
            lng: lng,
            height: height,
            heading: heading,
            pitch: pitch,
            roll: roll
        }, {
            duration: 1.5 // 1.5秒かけてスムーズにカメラをシンクロ移動
        });
        console.log("[Kasugai Re:Earth] カメラ同期移動を実行完了。");
    }
});
