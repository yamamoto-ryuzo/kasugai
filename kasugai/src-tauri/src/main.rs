// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{
    Manager, Position, Rect, Size, WebviewBuilder, WebviewUrl, WindowBuilder,
    PhysicalPosition, PhysicalSize,
};

// フロントエンドから呼び出されるRustコマンド
#[tauri::command]
fn get_system_info() -> String {
    "ステータス: 正常稼働中\nエンジン: Tauri v2 (Rust)\nWebview: Microsoft WebView2\n応答時間: リアルタイム".to_string()
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_system_info])
        .setup(|app| {
            // メインウィンドウを作成（Webviewなしの親ウィンドウ）
            let window = WindowBuilder::new(app, "main")
                .title("Kasugai 3-Split Viewer")
                .inner_size(1200.0, 800.0)
                .resizable(true)
                .build()?;

            // ウィンドウの初期物理サイズを取得
            let size = window.inner_size()?;
            let width = size.width as f64;
            let height = size.height as f64;

            // 3つのWebviewを横並びにする（横幅を3等分）
            let pane_width = width / 3.0;

            // 各Webviewのビルダーを作成
            // それぞれ index1.html, index2.html, index3.html を読み込むように設定
            let webview1_builder = WebviewBuilder::new("pane1", WebviewUrl::App("index1.html".into()));
            let webview2_builder = WebviewBuilder::new("pane2", WebviewUrl::App("index2.html".into()));
            let webview3_builder = WebviewBuilder::new("pane3", WebviewUrl::App("index3.html".into()));

            // ウィンドウにWebviewを子として追加
            let _wv1 = window.add_child(
                webview1_builder,
                PhysicalPosition::new(0, 0),
                PhysicalSize::new(pane_width as u32, height as u32),
            )?;

            let _wv2 = window.add_child(
                webview2_builder,
                PhysicalPosition::new(pane_width as i32, 0),
                PhysicalSize::new(pane_width as u32, height as u32),
            )?;

            let _wv3 = window.add_child(
                webview3_builder,
                PhysicalPosition::new((pane_width * 2.0) as i32, 0),
                PhysicalSize::new(pane_width as u32, height as u32),
            )?;

            // ウィンドウのリサイズイベントを監視し、3つのWebviewの境界（bounds）を更新
            let window_clone = window.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::Resized(new_size) = event {
                    let w = new_size.width as f64;
                    let h = new_size.height as f64;
                    let pw = w / 3.0;

                    if let Some(wv1) = window_clone.get_webview("pane1") {
                        let _ = wv1.set_bounds(Rect {
                            position: Position::Physical(PhysicalPosition::new(0, 0)),
                            size: Size::Physical(PhysicalSize::new(pw as u32, h as u32)),
                        });
                    }
                    if let Some(wv2) = window_clone.get_webview("pane2") {
                        let _ = wv2.set_bounds(Rect {
                            position: Position::Physical(PhysicalPosition::new(pw as i32, 0)),
                            size: Size::Physical(PhysicalSize::new(pw as u32, h as u32)),
                        });
                    }
                    if let Some(wv3) = window_clone.get_webview("pane3") {
                        let _ = wv3.set_bounds(Rect {
                            position: Position::Physical(PhysicalPosition::new((pw * 2.0) as i32, 0)),
                            size: Size::Physical(PhysicalSize::new(pw as u32, h as u32)),
                        });
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
