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
    pane3_current_host: Mutex<Option<String>>,
    // false: pane2=center, pane3=right  |  true: pane3=center, pane2=right
    pane_swapped: Mutex<bool>,
}

// 指定したドメイン・識別名に合致するサービス情報を自動で取得し、対象WebviewへJSコードを注入して自動入力する
#[tauri::command]
fn autofill_credentials(
    app_handle: tauri::AppHandle,
    target_panes: Vec<String>, // ["pane2", "pane3"] などの注入したいWebview名
    service: String,
    username: String,
) -> Result<(), String> {
    let entry = keyring::Entry::new(&service, &username)
        .map_err(|e| e.to_string())?;
    
    let password = entry.get_password().map_err(|e| e.to_string())?;

    if let Some(window) = app_handle.get_window("main") {
        for pane in target_panes {
            if let Some(wv) = window.get_webview(&pane) {
                // BOXなどのログイン画面にある一般的な input[type="email"], input[type="password"] や name="login", name="password" 等をターゲットにします
                let exec_js = format!(
                    r#"
                    (function() {{
                        function fill() {{
                            // BOXや一般サイトで「検索窓」「その他の入力欄」に誤ってユーザー名(メールアドレス等)を入れてしまわないよう、ターゲットを厳格化
                            var emailInputs = document.querySelectorAll(
                                'input[type="email"], ' +
                                'input[name="login"]:not([type="hidden"]), ' +
                                'input[name="username"]:not([type="hidden"]), ' +
                                'input[id*="username"]:not([type="hidden"]), ' +
                                'input[id*="login"]:not([type="hidden"]), ' +
                                'input#login-email, ' +
                                '.login-field input[type="text"]'
                            );
                            var passInputs = document.querySelectorAll(
                                'input[type="password"], ' +
                                'input[name*="pass"]:not([type="hidden"]), ' +
                                'input[id*="password"]:not([type="hidden"])'
                            );
                            
                            var filled = false;
                            
                            // IDの自動入力（空であるか、プレースホルダー状態、もしくはクリア状態のみ挿入）
                            for (var i = 0; i < emailInputs.length; i++) {{
                                var el = emailInputs[i];
                                // 検索窓（search等）ではないことを追加検証
                                if (el && (el.placeholder || "").toLowerCase().indexOf("search") === -1 && el.getAttribute("role") !== "searchbox") {{
                                    el.value = {username_js};
                                    el.dispatchEvent(new Event('input', {{ bubbles: true }}));
                                    el.dispatchEvent(new Event('change', {{ bubbles: true }}));
                                    filled = true;
                                }}
                            }}
                            
                            // パスワードの自動入力
                            for (var j = 0; j < passInputs.length; j++) {{
                                var el = passInputs[j];
                                if (el) {{
                                    el.value = {password_js};
                                    el.dispatchEvent(new Event('input', {{ bubbles: true }}));
                                    el.dispatchEvent(new Event('change', {{ bubbles: true }}));
                                    filled = true;
                                }}
                            }}
                            return filled;
                        }}
                        
                        // 念のため即座に実行するが、SPAなどで遅れてフォームが表示される場合を想定して監視も行う
                        if (!fill()) {{
                            var attempts = 0;
                            var interval = setInterval(function() {{
                                attempts++;
                                if (fill() || attempts > 10) {{
                                    clearInterval(interval);
                                }}
                            }}, 500);
                        }}
                    }})();
                    "#,
                    username_js = serde_json::to_string(&username).unwrap(),
                    password_js = serde_json::to_string(&password).unwrap()
                );
                
                // WebView2 にJavaScriptを注入して自動入力
                let _ = wv.eval(&exec_js);
            }
        }
    }
    
    Ok(())
}

// OS資格情報マネージャー（keyringクレート）を使ったセキュリティの高いID/PW保存コマンド
#[tauri::command]
fn save_credentials(service: String, username: String, password: Option<String>) -> Result<(), String> {
    let entry = keyring::Entry::new(&service, &username)
        .map_err(|e| e.to_string())?;
    
    if let Some(pw) = password {
        entry.set_password(&pw).map_err(|e| e.to_string())?;
    } else {
        // パスワードが渡されなければIDのみ記憶（空パスワードもしくはプレースホルダ）
        entry.set_password("").map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn get_credentials(service: String, username: String) -> Result<String, String> {
    let entry = keyring::Entry::new(&service, &username)
        .map_err(|e| e.to_string())?;
    
    entry.get_password().map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_credentials(service: String, username: String) -> Result<(), String> {
    let entry = keyring::Entry::new(&service, &username)
        .map_err(|e| e.to_string())?;
    
    entry.delete_password().map_err(|e| e.to_string())?;
    Ok(())
}

// フロントエンドから呼び出されるRustコマンド
#[tauri::command]
fn get_system_info() -> String {
    "ステータス: 正常稼働中\nエンジン: Tauri v2 (Rust)\nWebview: Microsoft WebView2\n応答時間: リアルタイム".to_string()
}

// 画面1(左)から送信されたURLを中央のWebviewで開く（物理的位置に追従）
#[tauri::command]
fn open_in_pane2(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, SplitterState>,
    url: String,
) {
    let swapped = *state.pane_swapped.lock().unwrap();
    if let Ok(target_url) = tauri::Url::parse(&url) {
        let host = target_url.host_str().map(|h| h.to_string());

        if let Some(window) = app_handle.get_window("main") {
            if !swapped {
                // 通常時：pane2が中央
                if let Some(h) = host {
                    *state.pane2_current_host.lock().unwrap() = Some(h);
                }
                if let Some(wv2) = window.get_webview("pane2") {
                    let _ = wv2.navigate(target_url);
                }
            } else {
                // スワップ時：pane3が中央
                if let Some(h) = host {
                    *state.pane3_current_host.lock().unwrap() = Some(h);
                }
                if let Some(wv3) = window.get_webview("pane3") {
                    let _ = wv3.navigate(target_url);
                }
            }
        }
    }
}

// 画面1(左)または割り込みナビゲーションから送信されたURLを右のWebviewで開く（物理的位置に追従）
#[tauri::command]
fn open_in_pane3(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, SplitterState>,
    url: String,
) {
    let swapped = *state.pane_swapped.lock().unwrap();
    if let Some(window) = app_handle.get_window("main") {
        if !swapped {
            // 通常時：pane3が右
            if let Some(wv3) = window.get_webview("pane3") {
                if let Ok(target_url) = tauri::Url::parse(&url) {
                    let _ = wv3.navigate(target_url);
                }
            }
        } else {
            // スワップ時：pane2が右
            if let Some(wv2) = window.get_webview("pane2") {
                if let Ok(target_url) = tauri::Url::parse(&url) {
                    let _ = wv2.navigate(target_url);
                }
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
            let swapped = *state.pane_swapped.lock().unwrap();
            recalculate_webview_bounds(&window, w, h, ratio1, ratio2, swapped);
        }
    }
}

// ウィンドウのサイズ、比率に基づいてWebviewの境界を再設定するヘルパー
fn recalculate_webview_bounds(window: &tauri::Window, w: f64, h: f64, ratio1: f64, ratio2: f64, swapped: bool) {
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

    // pane2/pane3 の配置設定（swapped により中央/右を入れ替え）
    if !swapped {
        // 通常: pane2 = 中央, pane3 = 右
        if let Some(wv2) = window.get_webview("pane2") {
            let start_x = (x1 + sh) as i32;
            let width = ((x2 - sh) - (x1 + sh)).max(0.0) as u32;
            let _ = wv2.set_bounds(Rect {
                position: Position::Physical(PhysicalPosition::new(start_x, 0)),
                size: Size::Physical(PhysicalSize::new(width, h as u32)),
            });
        }

        if let Some(wv3) = window.get_webview("pane3") {
            let start_x = (x2 + sh) as i32;
            let width = (w - (x2 + sh)).max(0.0) as u32;
            let _ = wv3.set_bounds(Rect {
                position: Position::Physical(PhysicalPosition::new(start_x, 0)),
                size: Size::Physical(PhysicalSize::new(width, h as u32)),
            });
        }
    } else {
        // 反転: pane3 = 中央, pane2 = 右
        if let Some(wv3) = window.get_webview("pane3") {
            let start_x = (x1 + sh) as i32;
            let width = ((x2 - sh) - (x1 + sh)).max(0.0) as u32;
            let _ = wv3.set_bounds(Rect {
                position: Position::Physical(PhysicalPosition::new(start_x, 0)),
                size: Size::Physical(PhysicalSize::new(width, h as u32)),
            });
        }

        if let Some(wv2) = window.get_webview("pane2") {
            let start_x = (x2 + sh) as i32;
            let width = (w - (x2 + sh)).max(0.0) as u32;
            let _ = wv2.set_bounds(Rect {
                position: Position::Physical(PhysicalPosition::new(start_x, 0)),
                size: Size::Physical(PhysicalSize::new(width, h as u32)),
            });
        }
    }
}

// 中央ペインを pane2 にする / pane3 にする切替コマンド
#[tauri::command]
fn set_center(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, SplitterState>,
    center: String,
) {
    let mut swapped = state.pane_swapped.lock().unwrap();
    *swapped = match center.as_str() {
        "pane2" => false,
        "pane3" => true,
        _ => *swapped,
    };

    if let Some(window) = app_handle.get_window("main") {
        if let Ok(size) = window.inner_size() {
            let w = size.width as f64;
            let h = size.height as f64;
            let r1 = *state.ratio1.lock().unwrap();
            let r2 = *state.ratio2.lock().unwrap();
            recalculate_webview_bounds(&window, w, h, r1, r2, *swapped);
        }
    }
}

fn main() {
    tauri::Builder::default()
        .manage(SplitterState {
            ratio1: Mutex::new(0.1),
            ratio2: Mutex::new(0.8),
            pane2_current_host: Mutex::new(None),
            pane3_current_host: Mutex::new(None),
            pane_swapped: Mutex::new(false),
        })
        .invoke_handler(tauri::generate_handler![
            get_system_info,
            update_splitter,
            open_in_pane2,
            open_in_pane3,
            set_center,
            save_credentials,
            get_credentials,
            delete_credentials,
            autofill_credentials
        ])
        .setup(|app| {
            // メインウィンドウを作成
            let window = WindowBuilder::new(app, "main")
                .title("Kasugai 3-Split Viewer")
                .inner_size(1200.0, 800.0)
                .resizable(true)
                .maximized(true)
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
            
            let app_handle_for_nav2 = app.handle().clone();
            let app_handle_for_new_window2 = app.handle().clone();
            let webview2_builder = WebviewBuilder::new("pane2", WebviewUrl::App("index2.html".into()))
                .on_navigation(move |url| {
                    let url_str = url.as_str();

                    // ローカルの読み込みは常に許可
                    if url_str.starts_with("tauri://") || url_str.contains("localhost") || url_str.contains("index2.html") {
                        return true;
                    }

                    let state = app_handle_for_nav2.state::<SplitterState>();
                    let swapped = *state.pane_swapped.lock().unwrap();

                    // 通常時はpane2が中央（ホワイトリストによる判定）
                    // swapped時はpane2が右面となる（右面としての動作：基本的には自身の内側ナビゲーションは100%許可等）
                    if !swapped {
                        let allowed_host_opt = state.pane2_current_host.lock().unwrap().clone();
                        if let Some(target_host) = url.host_str() {
                            if let Some(allowed_host) = allowed_host_opt {
                                if target_host == allowed_host || target_host.ends_with(&format!(".{}", allowed_host)) {
                                    return true;
                                }
                            }
                        }

                        // 外部ドメインの場合は、右側のWebview (pane3) でひらく
                        if let Some(window) = app_handle_for_nav2.get_window("main") {
                            if let Some(wv3) = window.get_webview("pane3") {
                                if let Ok(target_url) = tauri::Url::parse(url_str) {
                                    let _ = wv3.navigate(target_url);
                                    return false; // 画面2側の遷移をブロック
                                }
                            }
                        }
                    }

                    true
                })
                .on_new_window(move |url, _new_window| {
                    let url_str = url.as_str();
                    let state = app_handle_for_new_window2.state::<SplitterState>();
                    let swapped = *state.pane_swapped.lock().unwrap();
                    
                    if let Some(window) = app_handle_for_new_window2.get_window("main") {
                        if !swapped {
                            // 通常時：中央(pane2)から新しく開くURLは右(pane3)へルーティング
                            if let Some(wv3) = window.get_webview("pane3") {
                                if let Ok(target_url) = tauri::Url::parse(url_str) {
                                    let _ = wv3.navigate(target_url);
                                }
                            }
                        } else {
                            // swapped時：右に追いやられたpane2で新ウィンドウが発生した場合
                            // 本来の右画面の役割に準じ、別のポップアップを制限するかそのまま何もしない
                        }
                    }
                    
                    tauri::webview::NewWindowResponse::Deny
                });

            // pane3 (通常時は右、swapped時は中央) の挙動も役割交代に対応
            let app_handle_for_nav3 = app.handle().clone();
            let app_handle_for_new_window3 = app.handle().clone();
            let webview3_builder = WebviewBuilder::new("pane3", WebviewUrl::App("index3.html".into()))
                .on_navigation(move |url| {
                    let url_str = url.as_str();

                    if url_str.starts_with("tauri://") || url_str.contains("localhost") || url_str.contains("index3.html") {
                        return true;
                    }

                    let state = app_handle_for_nav3.state::<SplitterState>();
                    let swapped = *state.pane_swapped.lock().unwrap();

                    // swapped（画面3が中央にいる）場合、通常時に画面2（中央）が持っていた「ドメイン制限・他リンクをもう一方の別ペインに送る」役割を担当する
                    if swapped {
                        let allowed_host_opt = state.pane3_current_host.lock().unwrap().clone();
                        if let Some(target_host) = url.host_str() {
                            if let Some(allowed_host) = allowed_host_opt {
                                if target_host == allowed_host || target_host.ends_with(&format!(".{}", allowed_host)) {
                                    return true;
                                }
                            }
                        }

                        // 中央（この時はpane3）から見た外部ドメインへのリンクは、右側（この時はpane2）で開かせる
                        if let Some(window) = app_handle_for_nav3.get_window("main") {
                            if let Some(wv2) = window.get_webview("pane2") {
                                if let Ok(target_url) = tauri::Url::parse(url_str) {
                                    let _ = wv2.navigate(target_url);
                                    return false; // 中央(pane3)側の遷移をブロック
                                }
                            }
                        }
                    }

                    true
                })
                .on_new_window(move |url, _new_window| {
                    let url_str = url.as_str();
                    let state = app_handle_for_new_window3.state::<SplitterState>();
                    let swapped = *state.pane_swapped.lock().unwrap();
                    
                    if let Some(window) = app_handle_for_new_window3.get_window("main") {
                        if swapped {
                            // swapped（中央がpane3）の場合、新たなURLは右側（pane2）へルーティング
                            if let Some(wv2) = window.get_webview("pane2") {
                                if let Ok(target_url) = tauri::Url::parse(url_str) {
                                    let _ = wv2.navigate(target_url);
                                }
                            }
                        }
                    }
                    
                    tauri::webview::NewWindowResponse::Deny
                });

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

            // 初期のスプリッター比率(1:7:2)をベースに各Webviewのサイズ・位置をセット
            let swapped_init = *app.state::<SplitterState>().pane_swapped.lock().unwrap();
            recalculate_webview_bounds(&window, width, height, 0.1, 0.8, swapped_init);

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

                    let swapped_now = *state.pane_swapped.lock().unwrap();
                    recalculate_webview_bounds(&window_clone, w, h, r1, r2, swapped_now);
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
