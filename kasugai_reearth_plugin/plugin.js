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

        // 1. KASUGAI(Tauri)の最外層Webviewから送られてくる W3C標準 postMessage を待ち受ける
        window.addEventListener("message", (event) => {
            const data = event.data;
            if (data && data.action === "sync_camera") {
                const { lat, lon, height } = data.payload;
                console.log("[Kasugai Re:Earth Plugin] カメラ同期命令を検知しました:", lat, lon, height);

                // 2. 隔離されたプラグインiframeから、Re:Earthコアプログラム（親Window）へメッセージを中継
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

// 1. Re:EarthのWidget UIとして、上記のHTML(メッセージ中継ブリッジ)を描画・常駐させる
reearth.ui.show(widget_html, { visible: true });

// 2. widget.html(iframe)から送られてきた中継メッセージを待ち受け、Re:Earth公式のカメラ同期APIを叩く
reearth.on("message", (msg) => {
    if (msg && msg.action === "reearth_flyto") {
        const { lat, lng, height, heading, pitch, roll } = msg.camera;
        
        // Re:Earth公式カメラ操作APIをコールして位置移動！
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
        console.log("[Kasugai Re:Earth] カメラ同期移動に成功しました:", lat, lng, height);
    }
});
