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
    pane2_current_host: Mutex<Option<String>>,
}

// フロントエンドから呼び出されるRustコマンド
#[tauri::command]
fn get_system_info() -> String {
    "ステータス: 正常稼働中\nエンジン: Tauri v2 (Rust)\nWebview: Microsoft WebView2\n応答時間: リアルタイム".to_string()
}

// 画面1(左)から送信されたURLを画面2(中央)のWebviewで開く
#[tauri::command]
fn open_in_pane2(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, SplitterState>,
    url: String,
) {
    if let Ok(target_url) = tauri::Url::parse(&url) {
        // 画面1から指示されたURLのホスト名を保存
        if let Some(host) = target_url.host_str() {
            *state.pane2_current_host.lock().unwrap() = Some(host.to_string());
        }

        if let Some(window) = app_handle.get_window("main") {
            if let Some(wv2) = window.get_webview("pane2") {
                let _ = wv2.navigate(target_url);
            }
        }
    }
}

// 画面1(左)または割り込みナビゲーションから送信されたURLを画面3(右)のWebviewで開く
#[tauri::command]
fn open_in_pane3(app_handle: tauri::AppHandle, url: String) {
    if let Some(window) = app_handle.get_window("main") {
        if let Some(wv3) = window.get_webview("pane3") {
            if let Ok(target_url) = tauri::Url::parse(&url) {
                let _ = wv3.navigate(target_url);
            }
        }
    }
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
            pane2_current_host: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            get_system_info,
            update_splitter,
            open_in_pane2,
            open_in_pane3
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
            // 画面2(中央)のWebviewBuilderに、Rustネイティブのナビゲーション監視(on_navigation)を設定。
            // 外部ドメインでのJSセキュリティ制約を完全にバイパスし、Rust側で100%確実にインターセプトします。
            let app_handle_for_pane1 = app.handle().clone();
            let webview1_builder = WebviewBuilder::new("pane1", WebviewUrl::App("index1.html".into()))
                .on_new_window(move |url, _new_window| {
                    let url_str = url.as_str();
                    
                    if let Some(window) = app_handle_for_pane1.get_window("main") {
                        if let Some(wv3) = window.get_webview("pane3") {
                            if let Ok(target_url) = tauri::Url::parse(url_str) {
                                let _ = wv3.navigate(target_url);
                            }
                        }
                    }
                    
                    tauri::webview::NewWindowResponse::Deny
                });
            
            let app_handle_for_nav = app.handle().clone();
            let app_handle_for_new_window = app.handle().clone();
            let webview2_builder = WebviewBuilder::new("pane2", WebviewUrl::App("index2.html".into()))
                .on_navigation(move |url| {
                    let url_str = url.as_str();

                    // ローカルの読み込みは常に許可
                    if url_str.starts_with("tauri://") || url_str.contains("localhost") || url_str.contains("index2.html") {
                        return true;
                    }

                    // 画面1(左)からリクエストされた初期URLホスト、または現在のアクティブホストへの遷移は許可する
                    let state = app_handle_for_nav.state::<SplitterState>();
                    let allowed_host_opt = state.pane2_current_host.lock().unwrap().clone();

                    if let Some(target_host) = url.host_str() {
                        if let Some(allowed_host) = allowed_host_opt {
                            // 同一ホストまたはサブドメインへの遷移は画面2内で許可する
                            if target_host == allowed_host || target_host.ends_with(&format!(".{}", allowed_host)) {
                                return true;
                            }
                        }
                    }

                    // 完全に外部のドメイン、または別サイトのリンクがクリックされた場合は、
                    // 画面2での遷移をキャンセル(false)し、画面3(右)でそのURLを開く！
                    if let Some(window) = app_handle_for_nav.get_window("main") {
                        if let Some(wv3) = window.get_webview("pane3") {
                            if let Ok(target_url) = tauri::Url::parse(url_str) {
                                let _ = wv3.navigate(target_url);
                                return false; // 画面2側の遷移をブロック
                            }
                        }
                    }

                    true
                })
                // 画面2(中央)の新しいウィンドウ(target="_blank"等)の作成要求をインターセプトして画面3(右)で開く！
                .on_new_window(move |url, _new_window| {
                    let url_str = url.as_str();
                    
                    if let Some(window) = app_handle_for_new_window.get_window("main") {
                        if let Some(wv3) = window.get_webview("pane3") {
                            if let Ok(target_url) = tauri::Url::parse(url_str) {
                                let _ = wv3.navigate(target_url);
                            }
                        }
                    }
                    
                    // 新しい別ネイティブウィンドウの生成要求自体は却下(Deny)する
                    tauri::webview::NewWindowResponse::Deny
                });

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
