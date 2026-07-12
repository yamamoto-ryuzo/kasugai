console.log("[Kasugai Re:Earth Plugin] 最外層でのカメラ同期リスナーを開始しました。");

// 1. KASUGAI(Tauri/Rust)の最外層Webviewから送られてくる W3C標準 postMessage を直接リッスンする。
// （※iframe(ui.show)を介さないため、二重iframe隔離によるメッセージ不達バグを100%回避します）
window.addEventListener("message", (event) => {
    const data = event.data;
    if (data && data.action === "sync_camera") {
        const { lat, lon, height } = data.payload;
        console.log("[Kasugai Re:Earth Plugin] カメラ同期命令を受信しました:", lat, lon, height);

        // 2. 最外層コンテキストで実行されている Re:Earth公式カメラ操作APIをダイレクトにコール！
        reearth.visualizer.camera.flyTo({
            lat: lat,
            lng: lon,
            height: height,
            heading: 0,
            pitch: -1.5, // 真下を見下ろすアングル
            roll: 0
        }, {
            duration: 1.5 // 1.5秒かけてスムーズに同期移動
        });
        console.log("[Kasugai Re:Earth] カメラ同期移動を実行完了。");
    }
});
