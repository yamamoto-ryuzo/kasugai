// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;
use tauri::{
    Manager, Position, Rect, Size, WebviewBuilder, WebviewUrl, WindowBuilder,
    PhysicalPosition, PhysicalSize,
};

// スプリッターの比率を保持するグローバルステート
struct SplitterState {
    ratio1: Mutex<f64>,
    ratio2: Mutex<f64>,
}

// フロントエンドから呼び出されるRustコマンド
#[tauri::command]
fn get_system_info() -> String {
    "ステータス: 正常稼働中\nエンジン: Tauri v2 (Rust)\nWebview: Microsoft WebView2\n応答時間: リアルタイム".to_string()
}

// ドラッグ中にスプリッター比率を更新し、各Webviewの境界（サイズ）を再計算して配置する
#[tauri::command]
fn update_splitter(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, SplitterState>,
    ratio1: f64,
    ratio2: f64,
) {
    *state.ratio1.lock().unwrap() = ratio1;
    *state.ratio2.lock().unwrap() = ratio2;

    if let Some(window) = app_handle.get_window("main") {
        if let Ok(size) = window.inner_size() {
            let w = size.width as f64;
            let h = size.height as f64;
            recalculate_webview_bounds(&window, w, h, ratio1, ratio2);
        }
    }
}

// ウィンドウのサイズ、比率に基づいてWebviewの境界を再設定するヘルパー
fn recalculate_webview_bounds(window: &tauri::Window, w: f64, h: f64, ratio1: f64, ratio2: f64) {
    let splitter_width = 8.0;
    let sh = splitter_width / 2.0;

    let x1 = w * ratio1;
    let x2 = w * ratio2;

    // ベースWebview（index.html）は常に全画面
    if let Some(base_wv) = window.get_webview("main_webview") {
        let _ = base_wv.set_bounds(Rect {
            position: Position::Physical(PhysicalPosition::new(0, 0)),
            size: Size::Physical(PhysicalSize::new(w as u32, h as u32)),
        });
    }

    // pane1 (左) の配置設定
    if let Some(wv1) = window.get_webview("pane1") {
        let width = (x1 - sh).max(0.0) as u32;
        let _ = wv1.set_bounds(Rect {
            position: Position::Physical(PhysicalPosition::new(0, 0)),
            size: Size::Physical(PhysicalSize::new(width, h as u32)),
        });
    }

    // pane2 (中央) の配置設定
    if let Some(wv2) = window.get_webview("pane2") {
        let start_x = (x1 + sh) as i32;
        let width = ((x2 - sh) - (x1 + sh)).max(0.0) as u32;
        let _ = wv2.set_bounds(Rect {
            position: Position::Physical(PhysicalPosition::new(start_x, 0)),
            size: Size::Physical(PhysicalSize::new(width, h as u32)),
        });
    }

    // pane3 (右) の配置設定
    if let Some(wv3) = window.get_webview("pane3") {
        let start_x = (x2 + sh) as i32;
        let width = (w - (x2 + sh)).max(0.0) as u32;
        let _ = wv3.set_bounds(Rect {
            position: Position::Physical(PhysicalPosition::new(start_x, 0)),
            size: Size::Physical(PhysicalSize::new(width, h as u32)),
        });
    }
}

fn main() {
    tauri::Builder::default()
        .manage(SplitterState {
            ratio1: Mutex::new(0.3333),
            ratio2: Mutex::new(0.6667),
        })
        .invoke_handler(tauri::generate_handler![
            get_system_info,
            update_splitter
        ])
        .setup(|app| {
            // メインウィンドウを作成
            let window = WindowBuilder::new(app, "main")
                .title("Kasugai 3-Split Viewer")
                .inner_size(1200.0, 800.0)
                .resizable(true)
                .build()?;

            let size = window.inner_size()?;
            let width = size.width as f64;
            let height = size.height as f64;

            // スプリッターを表示するベースWebview (main_webview) の作成
            let base_webview_builder = WebviewBuilder::new("main_webview", WebviewUrl::App("index.html".into()));
            let _base_wv = window.add_child(
                base_webview_builder,
                PhysicalPosition::new(0, 0),
                PhysicalSize::new(width as u32, height as u32),
            )?;

            // 3つの子Webviewをマウント
            let webview1_builder = WebviewBuilder::new("pane1", WebviewUrl::App("index1.html".into()));
            let webview2_builder = WebviewBuilder::new("pane2", WebviewUrl::App("index2.html".into()));
            let webview3_builder = WebviewBuilder::new("pane3", WebviewUrl::App("index3.html".into()));

            // pane1, pane2, pane3 をベースウィンドウの子Webviewとして追加
            let _wv1 = window.add_child(
                webview1_builder,
                PhysicalPosition::new(0, 0),
                PhysicalSize::new(0, 0), // 最初は0サイズで仮生成、直後にリサイズ処理で配置
            )?;

            let _wv2 = window.add_child(
                webview2_builder,
                PhysicalPosition::new(0, 0),
                PhysicalSize::new(0, 0),
            )?;

            let _wv3 = window.add_child(
                webview3_builder,
                PhysicalPosition::new(0, 0),
                PhysicalSize::new(0, 0),
            )?;

            // 初期のスプリッター比率(1/3, 2/3)をベースに各Webviewのサイズ・位置をセット
            recalculate_webview_bounds(&window, width, height, 0.3333, 0.6667);

            // ウィンドウのリサイズイベントを監視し、3つのWebviewの境界（bounds）を最新 of 比率で再計算
            let window_clone = window.clone();
            let app_handle = app.handle().clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::Resized(new_size) = event {
                    let w = new_size.width as f64;
                    let h = new_size.height as f64;

                    // Stateから最新の比率を取得して再配置
                    let state = app_handle.state::<SplitterState>();
                    let r1 = *state.ratio1.lock().unwrap();
                    let r2 = *state.ratio2.lock().unwrap();

                    recalculate_webview_bounds(&window_clone, w, h, r1, r2);
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
