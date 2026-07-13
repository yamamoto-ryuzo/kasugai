// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;
use tauri::{
    Manager, Position, Rect, Size, WebviewBuilder, WebviewUrl, WindowBuilder,
    PhysicalPosition, PhysicalSize, Listener
};

use enigo::{Enigo, KeyboardControllable, Key};
use std::thread;
use std::time::Duration;

// スプリッターの比率を保持するグローバルステート
struct SplitterState {
    ratio1: Mutex<f64>,
    ratio2: Mutex<f64>,
    saved_ratios: Mutex<Option<(f64, f64)>>,
    pane2_current_host: Mutex<Option<String>>,
    reearth_email: Mutex<Option<String>>,
    box_email: Mutex<Option<String>>,
    active_pane2: Mutex<String>,
    pane3_active_tab: Mutex<String>,
    pane3_tabs: Mutex<Vec<String>>,
    tab_counter: Mutex<u64>,
}

use keyring::Entry;

// フロントエンドから呼び出されるRustコマンド
#[tauri::command]
fn get_system_info() -> String {
    "ステータス: 正常稼働中\nエンジン: Tauri v2 (Rust)\nWebview: Microsoft WebView2\n応答時間: リアルタイム".to_string()
}

// Keyringを利用したセキュアな資格情報の保存・取得・削除
#[tauri::command]
fn save_credential(
    state: tauri::State<'_, SplitterState>,
    service: String, 
    username: String, 
    password: String
) -> Result<(), String> {
    if service == "Kasugai_Reearth" {
        *state.reearth_email.lock().unwrap() = Some(username.clone());
    } else if service == "Kasugai_Box" {
        *state.box_email.lock().unwrap() = Some(username.clone());
    }
    let entry = Entry::new(&service, &username).map_err(|e| e.to_string())?;
    entry.set_password(&password).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_credential(service: String, username: String) -> Result<String, String> {
    let entry = Entry::new(&service, &username).map_err(|e| e.to_string())?;
    entry.get_password().map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_credential(service: String, username: String) -> Result<(), String> {
    let entry = Entry::new(&service, &username).map_err(|e| e.to_string())?;
    entry.delete_password().map_err(|e| e.to_string())
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
struct AutologinCreds {
    email: String,
    password: String,
}

// ==========================================
// BOX専用：DOM直接操作方式
// ==========================================
fn inject_box_autologin(wv: &tauri::Webview<tauri::Wry>, creds: AutologinCreds) {
    let script = format!(
        r#"
        (function() {{
            console.log("[Kasugai BOX] 自動ログインスクリプト開始");
            const email = "{}";
            const password = "{}";
            function attemptLogin() {{
                let emailInput = document.querySelector('input[type="email"]') || document.querySelector('input[name="login"]');
                let passInput = document.querySelector('input[type="password"]') || document.querySelector('input[name="password"]');
                let submitBtn = document.querySelector('button[type="submit"]') || document.querySelector('#login-submit') || document.querySelector('button[name="login_submit"]');

                if (emailInput && !emailInput.value && submitBtn) {{
                    emailInput.value = email;
                    emailInput.dispatchEvent(new Event('input', {{ bubbles: true }}));
                    setTimeout(() => {{ if (submitBtn) submitBtn.click(); }}, 300);
                }} else if (passInput && !passInput.value && submitBtn) {{
                    passInput.value = password;
                    passInput.dispatchEvent(new Event('input', {{ bubbles: true }}));
                    setTimeout(() => {{ if (submitBtn) submitBtn.click(); }}, 300);
                }}
            }}
            const timer = setInterval(attemptLogin, 1000);
            setTimeout(() => clearInterval(timer), 15000);
        }})();
        "#,
        creds.email.replace('"', "\\\""),
        creds.password.replace('"', "\\\"")
    );
    let _ = wv.eval(&script);
}

// ==========================================
// Re:Earth専用：物理タイピング方式 (Basic認証用)
// ==========================================
#[tauri::command]
fn type_credentials(email: String, password: String) {
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(500));
        let mut enigo = Enigo::new();
        enigo.key_sequence(&email);
        thread::sleep(Duration::from_millis(200));
        enigo.key_click(Key::Tab);
        thread::sleep(Duration::from_millis(200));
        enigo.key_sequence(&password);
        thread::sleep(Duration::from_millis(200));
        enigo.key_click(Key::Return);
    });
}

// ------------------------------------------
// BOX専用：開くコマンド
// ------------------------------------------
#[tauri::command]
fn open_box_in_pane(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, SplitterState>,
    _target: String, // "pane2" or "pane3" (無視して専用画面を使用)
    url: String,
    creds: AutologinCreds,
) {
    *state.active_pane2.lock().unwrap() = "box".to_string();
    update_splitter_internal(&app_handle, &state);
    let real_target = "pane2_box";

    if let Some(window) = app_handle.get_window("main") {
        if let Some(wv) = window.get_webview(real_target) {
            let target_url = tauri::Url::parse(&url).unwrap();
            
            let is_dedicated = real_target.starts_with("pane2_");
            let should_navigate = if let Ok(current_url) = wv.url() {
                if is_dedicated {
                    current_url.as_str() == "about:blank" || current_url.as_str().is_empty()
                } else {
                    current_url.host() != target_url.host() || current_url.as_str() == "about:blank"
                }
            } else {
                true
            };

            if should_navigate {
                let _ = wv.navigate(target_url);
            }
            let _ = wv.set_focus();
            
            if should_navigate {
                // 3段階ダメ押しインジェクション
                let wv_clone1 = wv.clone();
                let wv_clone2 = wv.clone();
                let wv_clone3 = wv.clone();
                let creds_clone1 = creds.clone();
                let creds_clone2 = creds.clone();
                let creds_clone3 = creds.clone();
                
                let app_handle_clone = app_handle.clone();
                inject_box_autologin(&wv_clone1, creds_clone1);
                
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(1800));
                    let _ = app_handle_clone.run_on_main_thread(move || {
                        inject_box_autologin(&wv_clone2, creds_clone2);
                    });
                    std::thread::sleep(std::time::Duration::from_millis(1700));
                    let _ = app_handle_clone.run_on_main_thread(move || {
                        inject_box_autologin(&wv_clone3, creds_clone3);
                    });
                });
            }
        }
    }
}

// ------------------------------------------
// Re:Earth専用：開くコマンド
// ------------------------------------------
#[tauri::command]
fn open_reearth_in_pane(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, SplitterState>,
    _target: String,
    url: String,
    creds: AutologinCreds,
) {
    *state.active_pane2.lock().unwrap() = "reearth".to_string();
    update_splitter_internal(&app_handle, &state);
    let real_target = "pane2_reearth";

    if let Some(window) = app_handle.get_window("main") {
        if let Some(wv) = window.get_webview(real_target) {
            let target_url = tauri::Url::parse(&url).unwrap();
            
            let is_dedicated = real_target.starts_with("pane2_");
            let should_navigate = if let Ok(current_url) = wv.url() {
                if is_dedicated {
                    current_url.as_str() == "about:blank" || current_url.as_str().is_empty()
                } else {
                    current_url.host() != target_url.host() || current_url.as_str() == "about:blank"
                }
            } else {
                true
            };

            if should_navigate {
                let _ = wv.navigate(target_url);
            }
            let _ = wv.set_focus();
            
            if should_navigate {
                // Basic認証ダイアログ待ち（またはページ表示後）に物理タイピング
                let c = creds.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(1500));
                    let mut enigo = Enigo::new();
                    enigo.key_sequence(&c.email);
                    thread::sleep(Duration::from_millis(100));
                    enigo.key_click(Key::Tab);
                    thread::sleep(Duration::from_millis(100));
                    enigo.key_sequence(&c.password);
                    thread::sleep(Duration::from_millis(100));
                    enigo.key_click(Key::Return);
                    println!("[Kasugai Reearth] Basic Auth typing completed.");
                });
            }
        }
    }
}

// 画面3用の動的タブ追加処理
fn add_pane3_tab(app_handle: tauri::AppHandle, target_url: tauri::Url) {
    let state = app_handle.state::<SplitterState>();
    let mut counter = state.tab_counter.lock().unwrap();
    *counter += 1;
    let tab_id = format!("pane3_tab_{}", *counter);
    
    state.pane3_tabs.lock().unwrap().push(tab_id.clone());
    *state.pane3_active_tab.lock().unwrap() = tab_id.clone();
    
    let url_str = target_url.as_str().to_string();
    let tab_id_clone = tab_id.clone();
    
    let app_handle_clone = app_handle.clone();
    
    let _ = app_handle.run_on_main_thread(move || {
        if let Some(window) = app_handle_clone.get_window("main") {
            let app_for_new = app_handle_clone.clone();
            
            let init_script = r#"
                window.addEventListener('dblclick', function(e) {
                    if (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke) {
                        window.__TAURI_INTERNALS__.invoke('pane_dblclick', { pane: 'pane3' });
                    } else if (window.__TAURI__ && window.__TAURI__.core) {
                        window.__TAURI__.core.invoke('pane_dblclick', { pane: 'pane3' });
                    }
                });
            "#;

            let builder = WebviewBuilder::new(&tab_id_clone, WebviewUrl::External(target_url))
                .initialization_script(init_script)
                .on_new_window(move |url, _new_window| {
                    if let Ok(target) = tauri::Url::parse(url.as_str()) {
                        add_pane3_tab(app_for_new.clone(), target);
                    }
                    tauri::webview::NewWindowResponse::Deny
                })
                .on_navigation(move |_url| {
                    true
                });
                
            let _wv = window.add_child(builder, PhysicalPosition::new(0, 0), PhysicalSize::new(0, 0));
            
            // フロントに通知してタブUIを作らせる
            let _ = window.emit("pane3_new_tab", serde_json::json!({
                "id": tab_id_clone,
                "url": url_str
            }));
            
            // Bounds再計算
            let state = app_handle_clone.state::<SplitterState>();
            update_splitter_internal(&app_handle_clone, &state);
        }
    });
}

// 通常のナビゲーション（Google Map等）
#[tauri::command]
fn open_in_pane2(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, SplitterState>,
    url: String,
) {
    if let Ok(target_url) = tauri::Url::parse(&url) {
        let host = target_url.host_str().map(|h| h.to_string());
        
        let target_str = if url.contains("earth.google.com") {
            *state.active_pane2.lock().unwrap() = "googleearth".to_string();
            update_splitter_internal(&app_handle, &state);
            "pane2_googleearth"
        } else if url.contains("google.com/maps") {
            *state.active_pane2.lock().unwrap() = "google".to_string();
            update_splitter_internal(&app_handle, &state);
            "pane2_google"
        } else if url.contains("box.com") {
            *state.active_pane2.lock().unwrap() = "box".to_string();
            update_splitter_internal(&app_handle, &state);
            "pane2_box"
        } else if url.contains("reearth.io") {
            *state.active_pane2.lock().unwrap() = "reearth".to_string();
            update_splitter_internal(&app_handle, &state);
            "pane2_reearth"
        } else if url.contains("map.yahoo.co.jp") {
            *state.active_pane2.lock().unwrap() = "yahoo".to_string();
            update_splitter_internal(&app_handle, &state);
            "pane2_yahoo"
        } else {
            *state.active_pane2.lock().unwrap() = "default".to_string();
            update_splitter_internal(&app_handle, &state);
            "pane2"
        };

        if target_str == "pane3" {
            add_pane3_tab(app_handle.clone(), target_url);
        } else {
            if let Some(window) = app_handle.get_window("main") {
                if let Some(wv) = window.get_webview(target_str) {
                    if let Some(h) = host { *state.pane2_current_host.lock().unwrap() = Some(h); }
                    
                    let mut is_googleearth_smooth = false;
                    let should_navigate = if let Ok(current_url) = wv.url() {
                        if target_str == "pane2_googleearth" && current_url.host_str() == Some("earth.google.com") {
                            is_googleearth_smooth = true;
                            false
                        } else {
                            // 現在のURLと異なる場合（位置情報などのパラメータ変更含む）は必ずナビゲートする
                            current_url.as_str() != target_url.as_str() && current_url.as_str() != format!("{}/", target_url.as_str())
                        }
                    } else {
                        true
                    };
                    
                    if should_navigate {
                        let _ = wv.navigate(target_url);
                    } else if is_googleearth_smooth {
                        let js = format!(
                            r#"
                            (function() {{
                                window.history.pushState(null, null, "{}");
                                window.dispatchEvent(new PopStateEvent('popstate', {{ state: null }}));
                            }})();
                            "#,
                            target_url.as_str()
                        );
                        let _ = wv.eval(&js);
                    }
                    let _ = wv.set_focus();
                }
            }
        }
    }
}

#[tauri::command]
fn open_in_pane3(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, SplitterState>,
    url: String,
) {
    if let Ok(target_url) = tauri::Url::parse(&url) {
        let target_str = if url.contains("earth.google.com") {
            *state.active_pane2.lock().unwrap() = "googleearth".to_string();
            update_splitter_internal(&app_handle, &state);
            "pane2_googleearth"
        } else if url.contains("google.com/maps") {
            *state.active_pane2.lock().unwrap() = "google".to_string();
            update_splitter_internal(&app_handle, &state);
            "pane2_google"
        } else if url.contains("box.com") {
            *state.active_pane2.lock().unwrap() = "box".to_string();
            update_splitter_internal(&app_handle, &state);
            "pane2_box"
        } else if url.contains("reearth.io") {
            *state.active_pane2.lock().unwrap() = "reearth".to_string();
            update_splitter_internal(&app_handle, &state);
            "pane2_reearth"
        } else if url.contains("map.yahoo.co.jp") {
            *state.active_pane2.lock().unwrap() = "yahoo".to_string();
            update_splitter_internal(&app_handle, &state);
            "pane2_yahoo"
        } else {
            "pane3"
        };

        if target_str == "pane3" {
            add_pane3_tab(app_handle.clone(), target_url);
        } else {
            if let Some(window) = app_handle.get_window("main") {
                if let Some(wv) = window.get_webview(target_str) {
                    let mut is_googleearth_smooth = false;
                    let should_navigate = if let Ok(current_url) = wv.url() {
                        if target_str == "pane2_googleearth" && current_url.host_str() == Some("earth.google.com") {
                            is_googleearth_smooth = true;
                            false
                        } else {
                            current_url.as_str() != target_url.as_str() && current_url.as_str() != format!("{}/", target_url.as_str())
                        }
                    } else {
                        true
                    };
                    
                    if should_navigate {
                        let _ = wv.navigate(target_url);
                    } else if is_googleearth_smooth {
                        let js = format!(
                            r#"
                            (function() {{
                                window.history.pushState(null, null, "{}");
                                window.dispatchEvent(new PopStateEvent('popstate', {{ state: null }}));
                            }})();
                            "#,
                            target_url.as_str()
                        );
                        let _ = wv.eval(&js);
                    }
                    let _ = wv.set_focus();
                }
            }
        }
    }
}

#[tauri::command]
fn update_splitter(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, SplitterState>,
    ratio1: f64,
    ratio2: f64,
) {
    if ratio1 >= 0.0 { *state.ratio1.lock().unwrap() = ratio1; }
    if ratio2 >= 0.0 { *state.ratio2.lock().unwrap() = ratio2; }
    if ratio1 >= 0.0 || ratio2 >= 0.0 {
        *state.saved_ratios.lock().unwrap() = None;
    }
    update_splitter_internal(&app_handle, &state);
}

use tauri::Emitter;

#[tauri::command]
fn pane_dblclick(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, SplitterState>,
    pane: String,
) {
    // 独立したWebView(pane2_cesiumなど)からのダブルクリックは無視する
    if pane.starts_with("pane2_") {
        println!("[Kasugai Rust] Ignored dblclick from dedicated pane: {}", pane);
        return;
    }

    let mut saved = state.saved_ratios.lock().unwrap();
    
    if pane == "pane1" {
        let w = if let Some(window) = app_handle.get_window("main") {
            window.inner_size().map(|s| s.width as f64).unwrap_or(1200.0)
        } else {
            1200.0
        };
        let r1 = *state.ratio1.lock().unwrap();
        let r2 = *state.ratio2.lock().unwrap();
        let width1 = w * r1;

        if width1 < 120.0 {
            // すでに閉じている（スプリットだけ）の場合は、通常表示に戻す
            let target_r = if let Some((saved_r1, _)) = *saved {
                if saved_r1 >= 120.0 / w { saved_r1 } else { 0.1 }
            } else {
                0.1
            };
            *state.ratio1.lock().unwrap() = target_r;
            *saved = None;
        } else {
            // 通常表示されている場合は、閉じる（スプリットだけ、比率 0.0にする）
            *saved = Some((r1, r2));
            *state.ratio1.lock().unwrap() = 0.0;
        }
    } else {
        if saved.is_some() {
            // 復元
            let (r1, r2) = saved.unwrap();
            *state.ratio1.lock().unwrap() = r1;
            *state.ratio2.lock().unwrap() = r2;
            *saved = None;
        } else {
            // 現在の比率を保存して最大化
            let r1 = *state.ratio1.lock().unwrap();
            let r2 = *state.ratio2.lock().unwrap();
            *saved = Some((r1, r2));

            match pane.as_str() {
                "pane2" => {
                    *state.ratio1.lock().unwrap() = 0.0;
                    *state.ratio2.lock().unwrap() = 1.0;
                }
                "pane3" => {
                    *state.ratio1.lock().unwrap() = 0.0;
                    *state.ratio2.lock().unwrap() = 0.0;
                }
                _ => {}
            }
        }
    }
    
    update_splitter_internal(&app_handle, &state);

    let r1 = *state.ratio1.lock().unwrap();
    let r2 = *state.ratio2.lock().unwrap();
    let _ = app_handle.emit("update_splitter_ui", serde_json::json!({
        "ratio1": r1,
        "ratio2": r2
    }));
}

fn update_splitter_internal(app_handle: &tauri::AppHandle, state: &tauri::State<'_, SplitterState>) {
    if let Some(window) = app_handle.get_window("main") {
        if let Ok(size) = window.inner_size() {
            let w = size.width as f64;
            let h = size.height as f64;
            let r1 = *state.ratio1.lock().unwrap();
            let r2 = *state.ratio2.lock().unwrap();
            let active = state.active_pane2.lock().unwrap().clone();
            recalculate_webview_bounds(&window, w, h, r1, r2, &active, state);
        }
    }
}

fn recalculate_webview_bounds(window: &tauri::Window, w: f64, h: f64, ratio1: f64, ratio2: f64, active_pane2: &str, state: &SplitterState) {
    let splitter_width = 8.0;
    let sh = splitter_width / 2.0;
    // 画面1が開いている時（ratio1 != 0.0 の時）は、常に最小幅 80px で完全に固定する
    let x1 = if ratio1 == 0.0 { 0.0 } else { 80.0 + sh };
    let x2 = w * ratio2;
    let tab_height = 50.0; // 画面2上部のタブ領域の高さ

    let rect_hidden = Rect {
        position: Position::Physical(PhysicalPosition::new(-10000, -10000)),
        size: Size::Physical(PhysicalSize::new(1, 1)),
    };

    if let Some(base_wv) = window.get_webview("main_webview") {
        let _ = base_wv.set_bounds(Rect {
            position: Position::Physical(PhysicalPosition::new(0, 0)),
            size: Size::Physical(PhysicalSize::new(w as u32, h as u32)),
        });
    }
    if let Some(wv1) = window.get_webview("pane1") {
        if ratio1 == 0.0 {
            let _ = wv1.set_bounds(rect_hidden);
        } else {
            let _ = wv1.set_bounds(Rect {
                position: Position::Physical(PhysicalPosition::new(0, 0)),
                size: Size::Physical(PhysicalSize::new(80, h as u32)),
            });
        }
    }

    let rect_center = Rect {
        position: Position::Physical(PhysicalPosition::new((x1 + sh) as i32, 0)),
        size: Size::Physical(PhysicalSize::new(((x2 - sh) - (x1 + sh)).max(0.0) as u32, h as u32)),
    };
    let rect_center_dedicated = Rect {
        position: Position::Physical(PhysicalPosition::new((x1 + sh) as i32, tab_height as i32)),
        size: Size::Physical(PhysicalSize::new(((x2 - sh) - (x1 + sh)).max(0.0) as u32, (h - tab_height).max(0.0) as u32)),
    };
    let rect_right = Rect {
        position: Position::Physical(PhysicalPosition::new((x2 + sh) as i32, 0)),
        size: Size::Physical(PhysicalSize::new((w - (x2 + sh)).max(0.0) as u32, h as u32)),
    };
    let rect_right_dedicated = Rect {
        position: Position::Physical(PhysicalPosition::new((x2 + sh) as i32, tab_height as i32)),
        size: Size::Physical(PhysicalSize::new((w - (x2 + sh)).max(0.0) as u32, (h - tab_height).max(0.0) as u32)),
    };

    let pane2_rect = rect_center;
    let pane2_dedicated_rect = rect_center_dedicated;
    let pane3_rect = rect_right;
    let pane3_dedicated_rect = rect_right_dedicated;

    let update_pane2 = |id: &str, is_active: bool, is_dedicated: bool| {
        if let Some(wv) = window.get_webview(id) {
            // Webviewが現在このウィンドウに属している場合のみ Bounds を更新する
            if wv.window().label() == window.label() {
                if is_active {
                    let _ = wv.set_bounds(if is_dedicated { pane2_dedicated_rect } else { pane2_rect });
                } else {
                    let _ = wv.set_bounds(rect_hidden);
                }
            }
        }
    };

    // pane2 (ベース画面) は、常に表示しておく（アクティブかどうかに関わらず、背面またはタブ領域として表示する）
    // ただし、完全に非表示にするのではなく、pane2 は常に配置しておくことでタブ部分が見えるようにする
    if let Some(wv2) = window.get_webview("pane2") {
        let _ = wv2.set_bounds(pane2_rect);
    }
    
    // 画面2の専用画面（pane2_...）の配置設定
    // 通常時はメイン画面内に配置し、detachされた場合のみバウンズ計算から実質的に除外する運用
    let is_detached = |label: &str| {
        window.get_window(&format!("detached_pane2_{}", label)).is_some() 
        || window.get_window("dedicated_pane2").is_some() 
    };

    let active = active_pane2;
    update_pane2("pane2_box", active == "box" && !is_detached("box"), true);
    update_pane2("pane2_reearth", active == "reearth" && !is_detached("reearth"), true);
    update_pane2("pane2_google", active == "google" && !is_detached("google"), true);
    update_pane2("pane2_googleearth", active == "googleearth" && !is_detached("googleearth"), true);
    update_pane2("pane2_yahoo", active == "yahoo" && !is_detached("yahoo"), true);
    update_pane2("pane2_cesium", active == "cesium", true); 

    // pane3 (ベース画面、タブUI領域など用) は常に配置
    if let Some(wv3) = window.get_webview("pane3") {
        let _ = wv3.set_bounds(pane3_rect);
    }

    // 動的に追加された pane3 の各タブWebviewのBounds更新
    let pane3_active = state.pane3_active_tab.lock().unwrap().clone();
    let pane3_tabs_vec = state.pane3_tabs.lock().unwrap().clone();
    
    for tab_id in pane3_tabs_vec {
        if let Some(wv) = window.get_webview(&tab_id) {
            if tab_id == pane3_active {
                let _ = wv.set_bounds(pane3_dedicated_rect); // タブ領域(50px)の下に配置
            } else {
                let _ = wv.set_bounds(rect_hidden);
            }
        }
    }
}

#[tauri::command]
fn switch_pane3_tab(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, SplitterState>,
    tab: String,
) {
    if state.pane3_tabs.lock().unwrap().contains(&tab) || tab == "default" {
        *state.pane3_active_tab.lock().unwrap() = tab;
        update_splitter_internal(&app_handle, &state);
    }
}

#[tauri::command]
fn close_pane3_tab(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, SplitterState>,
    tab: String,
) {
    let mut tabs = state.pane3_tabs.lock().unwrap();
    if let Some(pos) = tabs.iter().position(|x| *x == tab) {
        tabs.remove(pos);
        
        let app_clone = app_handle.clone();
        let tab_id = tab.clone();
        let _ = app_handle.run_on_main_thread(move || {
            if let Some(window) = app_clone.get_window("main") {
                if let Some(wv) = window.get_webview(&tab_id) {
                    let _ = wv.close();
                }
            }
        });

        // 削除したタブがアクティブだった場合、別のタブをアクティブにする
        let mut active = state.pane3_active_tab.lock().unwrap();
        if *active == tab {
            if tabs.len() > 0 {
                // 右隣か、なければ最後のタブ
                let new_idx = if pos < tabs.len() { pos } else { tabs.len() - 1 };
                *active = tabs[new_idx].clone();
            } else {
                *active = "default".to_string(); // すべてのタブが消えたらdefaultに戻る
            }
            
            // UIに新しいアクティブタブを通知
            let _ = app_handle.emit("pane3_active_changed", serde_json::json!({
                "id": *active
            }));
        }
    }
    drop(tabs); // ロック解放
    update_splitter_internal(&app_handle, &state);
}

#[tauri::command]
fn switch_pane2_tab(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, SplitterState>,
    tab: String,
) {
    *state.active_pane2.lock().unwrap() = tab.clone();
    update_splitter_internal(&app_handle, &state);

    // 既に独立ウィンドウに移動している場合、そのウィンドウをフォーカスする
    let wv_id = if tab == "default" { "pane2".to_string() } else { format!("pane2_{}", tab) };
    if let Some(wv) = app_handle.get_webview(&wv_id) {
        let win = wv.window();
        if win.label() != "main" {
            let _ = win.set_focus();
        }
    }
}

#[tauri::command]
fn prefetch_basic_auth(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, SplitterState>,
    url: String,
    creds: AutologinCreds,
) {
    *state.reearth_email.lock().unwrap() = Some(creds.email.clone());
    if let Some(window) = app_handle.get_window("main") {
        let script = format!(
            r#"(async function() {{
                const basicAuthHeader = "Basic " + btoa(unescape(encodeURIComponent("{}:{}")));
                await fetch("{}", {{ method: "GET", headers: {{ "Authorization": basicAuthHeader }}, mode: 'no-cors' }});
            }})();"#,
            creds.email, creds.password, url
        );
        if let Some(wv) = window.get_webview("pane2_reearth") {
            // Re:Earthの自動タイピングが意図しない画面で発火するのを防ぐため、
            // 初回クリック時（open_reearth_in_pane）までナビゲーションを遅延させます。
            let _ = wv.eval(&script);
        }
        if let Some(wv3) = window.get_webview("pane3") {
            let _ = wv3.eval(&script);
        }
    }
}

#[tauri::command]
async fn detach_window(
    app_handle: tauri::AppHandle,
    label: String,
    url: String,
    title: String,
) -> Result<(), String> {
    let window_label = if label == "dedicated_pane2" { "dedicated_pane2".to_string() } else { format!("detached_{}", label) };
    
    // 1. 移動対象のWebviewを取得（既存のものがあれば）
    // label が "box" なら Webview ID は "pane2_box"
    let wv_id = if label == "dedicated_pane2" {
        // dedicated_pane2 の場合は URL やコンテキストから判断する必要があるが、
        // 現状の index2.html の呼び出し方に合わせるなら label そのものが ID かもしれない
        label.clone()
    } else if label.starts_with("pane2_") {
        label.clone()
    } else {
        format!("pane2_{}", label)
    };

    // すでにウィンドウが開いている場合
    if let Some(win) = app_handle.get_window(&window_label) {
        let _ = win.set_focus();
        
        // 既存のWebviewが別のところにあるなら移動させる（Reparent）
        if let Some(wv) = app_handle.get_webview(&wv_id) {
            let current_win = wv.window();
            if current_win.label() != window_label {
                let _ = wv.reparent(&win).map_err(|e| e.to_string())?;
            }
            let _ = wv.set_bounds(Rect {
                position: Position::Physical(PhysicalPosition::new(0, 0)),
                size: Size::Physical(win.inner_size().unwrap()),
            }).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }

    // 新しいウィンドウを作成
    let win = WindowBuilder::new(&app_handle, &window_label)
        .title(&title)
        .inner_size(800.0, 600.0)
        .build()
        .map_err(|e| e.to_string())?;

    // リサイズイベントのハンドラを追加
    let app_clone_resize = app_handle.clone();
    let wv_id_resize = wv_id.clone();
    win.on_window_event(move |event| {
        if let tauri::WindowEvent::Resized(size) = event {
            if let Some(wv) = app_clone_resize.get_webview(&wv_id_resize) {
                let _ = wv.set_bounds(Rect {
                    position: Position::Physical(PhysicalPosition::new(0, 0)),
                    size: Size::Physical(*size),
                });
            }
        }
    });

    // Webviewを移動または新規作成
    if let Some(wv) = app_handle.get_webview(&wv_id) {
        // 既存のWebviewを移動
        wv.reparent(&win).map_err(|e| e.to_string())?;
        wv.set_bounds(Rect {
            position: Position::Physical(PhysicalPosition::new(0, 0)),
            size: Size::Physical(win.inner_size().unwrap()),
        }).map_err(|e| e.to_string())?;
    } else {
        // なければ新規（通常はここに来ないはず）
        let wv_builder = WebviewBuilder::new(&wv_id, WebviewUrl::External(tauri::Url::parse(&url).unwrap()));
        let _ = win.add_child(wv_builder, PhysicalPosition::new(0, 0), win.inner_size().unwrap())
            .map_err(|e| e.to_string())?;
    }

    let app_clone = app_handle.clone();
    let wv_id_clone = wv_id.clone();
    win.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { .. } = event {
            // ウィンドウが閉じられたらメインウィンドウ（main）に戻す
            if let Some(main_win) = app_clone.get_window("main") {
                if let Some(wv) = app_clone.get_webview(&wv_id_clone) {
                    let _ = wv.reparent(&main_win);
                    
                    // Webview2の描画更新を確実にするため、一旦隠蔽領域に飛ばしてから戻す
                    let _ = wv.set_bounds(Rect {
                        position: Position::Physical(PhysicalPosition::new(-10000, -10000)),
                        size: Size::Physical(PhysicalSize::new(1, 1)),
                    });

                    // 戻した後に少し遅延を置いてBounds再計算をトリガー
                    let app_handle_inner = app_clone.clone();
                    let wv_id_inner = wv_id_clone.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(100));
                        let app_handle_for_run = app_handle_inner.clone();
                        let wv_id_for_run = wv_id_inner.clone();
                        let _ = app_handle_inner.run_on_main_thread(move || {
                            let state = app_handle_for_run.state::<SplitterState>();
                            update_splitter_internal(&app_handle_for_run, &state);
                            
                            // 強制リロードが必要な場合（描画停止対策）
                            if let Some(wv_now) = app_handle_for_run.get_webview(&wv_id_for_run) {
                                let _ = wv_now.eval("window.dispatchEvent(new Event('resize'));");
                            }
                        });
                    });
                }
            }
            let _ = app_clone.emit("window_restored", serde_json::json!({ "label": wv_id_clone }));
        }
    });

    Ok(())
}

#[tauri::command]
async fn get_pane2_url(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, SplitterState>,
) -> Result<String, String> {
    let active = state.active_pane2.lock().unwrap().clone();
        let target_str = match active.as_str() {
            "box" => "pane2_box",
            "reearth" => "pane2_reearth",
            "google" => "pane2_google",
            "googleearth" => "pane2_googleearth",
            "yahoo" => "pane2_yahoo",
            "cesium" => "pane2_cesium",
            _ => "pane2",
        };

    let (tx, rx) = std::sync::mpsc::channel();
    let app_clone = app_handle.clone();
    
    let _ = app_handle.run_on_main_thread(move || {
        let mut result = Err("Could not get URL".to_string());
        if let Some(window) = app_clone.get_window("main") {
            if let Some(wv) = window.get_webview(&target_str) {
                if let Ok(url) = wv.url() {
                    result = Ok(url.to_string());
                }
            }
        }
        let _ = tx.send(result);
    });

    rx.recv().unwrap_or(Err("Thread communication error".to_string()))
}

#[tauri::command]
fn preload_webview(app_handle: tauri::AppHandle, target: String, url: String) {
    if target == "pane2_reearth" || target == "pane2_box" {
        // Re:EarthとBOXは起動時の自動ログイン（タイピング/DOM操作）を確実にするためプレロード（裏読み）をスキップします。
        return;
    }
    if let Some(window) = app_handle.get_window("main") {
        if let Some(wv) = window.get_webview(&target) {
            if let Ok(current) = wv.url() {
                if current.as_str() == "about:blank" || current.as_str().is_empty() {
                    if let Ok(target_url) = tauri::Url::parse(&url) {
                        let _ = wv.navigate(target_url);
                    }
                }
            }
        }
    }
}

#[derive(serde::Serialize)]
struct GeminiResponse {
    text: String,
    prompt_tokens: Option<i64>,
    candidates_tokens: Option<i64>,
    total_tokens: Option<i64>,
}

#[tauri::command]
async fn call_gemini(prompt: String, model: Option<String>) -> Result<GeminiResponse, String> {
    let entry = keyring::Entry::new("Kasugai_Gemini", "apikey").map_err(|e| e.to_string())?;
    let api_key = entry.get_password().map_err(|_| "Gemini APIキーが設定されていません。システム設定画面でAPIキーを登録してください。".to_string())?;

    if api_key.trim().is_empty() {
        return Err("Gemini APIキーが設定されていません。システム設定画面でAPIキーを登録してください。".to_string());
    }

    let model_name = model.unwrap_or_else(|| "gemini-1.5-flash".to_string());
    let model_name = if model_name.trim().is_empty() {
        "gemini-1.5-flash".to_string()
    } else {
        model_name
    };

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model_name,
        api_key
    );

    let body = serde_json::json!({
        "contents": [{
            "parts": [{"text": prompt}]
        }]
    });

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("APIリクエスト送信に失敗しました: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_body = response.text().await.unwrap_or_default();
        return Err(format!("Gemini APIエラー (ステータス: {}): {}", status, error_body));
    }

    let json_resp: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("応答の解析に失敗しました: {}", e))?;

    let text = json_resp
        .pointer("/candidates/0/content/parts/0/text")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            format!(
                "APIの応答形式が想定と異なります: {}",
                serde_json::to_string_pretty(&json_resp).unwrap_or_default()
            )
        })?;

    let prompt_tokens = json_resp.pointer("/usageMetadata/promptTokenCount").and_then(|v| v.as_i64());
    let candidates_tokens = json_resp.pointer("/usageMetadata/candidatesTokenCount").and_then(|v| v.as_i64());
    let total_tokens = json_resp.pointer("/usageMetadata/totalTokenCount").and_then(|v| v.as_i64());

    Ok(GeminiResponse {
        text: text.to_string(),
        prompt_tokens,
        candidates_tokens,
        total_tokens,
    })
}

fn main() {
    tauri::Builder::default()
        .manage(SplitterState {
            ratio1: Mutex::new(0.1),
            ratio2: Mutex::new(0.8),
            saved_ratios: Mutex::new(None),
            pane2_current_host: Mutex::new(None),
            reearth_email: Mutex::new(None),
            box_email: Mutex::new(None),
            active_pane2: Mutex::new("default".to_string()),
            pane3_active_tab: Mutex::new("default".to_string()),
            pane3_tabs: Mutex::new(Vec::new()),
            tab_counter: Mutex::new(0),
        })
        .invoke_handler(tauri::generate_handler![
            get_system_info,
            update_splitter,
            pane_dblclick,
            open_in_pane2,
            open_in_pane3,
            open_box_in_pane,
            open_reearth_in_pane,
            save_credential,
            get_credential,
            delete_credential,
            prefetch_basic_auth,
            type_credentials,
            preload_webview,
            switch_pane2_tab,
            switch_pane3_tab,
            close_pane3_tab,
            get_pane2_url,
            call_gemini,
            detach_window
        ])
        .setup(|app| {
            // =================================================================
            // Re:Earth 位置同期イベントリスナー（CORS/二重iframeサンドボックス完全突破）
            // ポータル（index1.html）や他ペインから 'move_cesium'（共通位置変更）イベントが
            // 発行された際、裏側のRustバックエンド権限により、Re:Earthを表示しているWebview
            // に対し、eval 経由で直接 window.reearth.visualizer.camera.flyTo を実行します。
            // =================================================================
            let app_handle_for_sync = app.handle().clone();
            app.listen("move_cesium", move |event| {
                if let Ok(payload) = serde_json::from_str::<serde_json::Value>(event.payload()) {
                    let lat = payload.get("lat").and_then(|v| v.as_f64()).unwrap_or(35.6812);
                    let lon = payload.get("lon").and_then(|v| v.as_f64()).unwrap_or(139.7671);
                    let zoom = payload.get("zoom").and_then(|v| v.as_f64()).unwrap_or(15.0);
                    // ズームレベルから高度（メートル）への概算換算
                    let height = 20000000.0 / 2.0_f64.powf(zoom - 1.0);

                    let js_code = format!(
                        r#"
                        (function() {{
                            window.postMessage({{
                                action: "sync_camera",
                                payload: {{
                                    lat: {},
                                    lon: {},
                                    height: {}
                                }}
                            }}, "*");
                        }})();
                        "#,
                        lat, lon, height
                    );

                    // 全ウィンドウの全Webviewに対してブロードキャスト（Re:Earth以外にも届くが害はない）
                    for (_, window) in app_handle_for_sync.webview_windows() {
                        let _ = window.eval(&js_code);
                    }
                }
            });

            let window = WindowBuilder::new(app, "main")
                .title("Kasugai 3-Split Viewer")
                .inner_size(1200.0, 800.0)
                .resizable(true)
                .maximized(true)
                .build()?;

            let app_handle_for_resize = app.handle().clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::Resized(_) = event {
                    let state = app_handle_for_resize.state::<SplitterState>();
                    update_splitter_internal(&app_handle_for_resize, &state);
                }
            });

            let size = window.inner_size()?;
            let width = size.width as f64;
            let height = size.height as f64;
            let base_webview_builder = WebviewBuilder::new("main_webview", WebviewUrl::App("index.html".into()));
            let _base_wv = window.add_child(base_webview_builder, PhysicalPosition::new(0, 0), PhysicalSize::new(width as u32, height as u32))?;
            
            let init_script_pane1 = r#"
                window.addEventListener('dblclick', function(e) {
                    if (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke) {
                        window.__TAURI_INTERNALS__.invoke('pane_dblclick', { pane: 'pane1' });
                    } else if (window.__TAURI__ && window.__TAURI__.core) {
                        window.__TAURI__.core.invoke('pane_dblclick', { pane: 'pane1' });
                    }
                });
            "#;
            let init_script_pane2 = r#"
                window.addEventListener('dblclick', function(e) {
                    if (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke) {
                        window.__TAURI_INTERNALS__.invoke('pane_dblclick', { pane: 'pane2' });
                    } else if (window.__TAURI__ && window.__TAURI__.core) {
                        window.__TAURI__.core.invoke('pane_dblclick', { pane: 'pane2' });
                    }
                });
            "#;
            let init_script_pane3 = r#"
                window.addEventListener('dblclick', function(e) {
                    if (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke) {
                        window.__TAURI_INTERNALS__.invoke('pane_dblclick', { pane: 'pane3' });
                    } else if (window.__TAURI__ && window.__TAURI__.core) {
                        window.__TAURI__.core.invoke('pane_dblclick', { pane: 'pane3' });
                    }
                });
            "#;

            let app_handle_for_pane1 = app.handle().clone();
            let webview1_builder = WebviewBuilder::new("pane1", WebviewUrl::App("index1.html".into()))
                .initialization_script(init_script_pane1)
                .on_new_window(move |url, _new_window| {
                    let url_str = url.as_str();
                    if let Ok(target_url) = tauri::Url::parse(url_str) {
                        add_pane3_tab(app_handle_for_pane1.clone(), target_url);
                    }
                    tauri::webview::NewWindowResponse::Deny
                });

            let app_handle_for_nav2 = app.handle().clone();
            let app_handle_for_new_window2 = app.handle().clone();
            let webview2_builder = WebviewBuilder::new("pane2", WebviewUrl::App("index2.html".into()))
                .initialization_script(init_script_pane2)
                .on_navigation(move |url| {
                    let url_str = url.as_str();
                    if url_str.starts_with("tauri://") || url_str.contains("localhost") || url_str.contains("127.0.0.1") 
                       || url_str.contains("index2.html") || url_str.contains("index3.html") 
                       || url_str.contains("account.box.com") || url_str.contains("app.box.com") || url_str.contains("reearth.io") || url_str.contains("earth.google.com") {
                        return true;
                    }
                    let state = app_handle_for_nav2.state::<SplitterState>();
                    let allowed_host_opt = state.pane2_current_host.lock().unwrap().clone();
                    if let Some(target_host) = url.host_str() {
                        if let Some(allowed_host) = allowed_host_opt {
                            if target_host == allowed_host || target_host.ends_with(&format!(".{}", allowed_host)) {
                                return true;
                            }
                        }
                    }
                    if let Ok(target_url) = tauri::Url::parse(url_str) {
                        add_pane3_tab(app_handle_for_nav2.clone(), target_url);
                        return false;
                    }
                    true
                })
                .on_new_window(move |url, _new_window| {
                    let url_str = url.as_str();
                    if let Ok(target_url) = tauri::Url::parse(url_str) {
                        add_pane3_tab(app_handle_for_new_window2.clone(), target_url);
                    }
                    tauri::webview::NewWindowResponse::Deny
                });

            let webview3_builder = WebviewBuilder::new("pane3", WebviewUrl::App("index3.html".into()))
                .initialization_script(init_script_pane3)
                .on_navigation(move |url| {
                    let url_str = url.as_str();
                    if url_str.starts_with("tauri://") || url_str.contains("localhost") || url_str.contains("127.0.0.1") 
                       || url_str.contains("index2.html") || url_str.contains("index3.html") 
                       || url_str.contains("account.box.com") || url_str.contains("app.box.com") || url_str.contains("reearth.io") || url_str.contains("earth.google.com") {
                        return true;
                    }
                    true
                })
                .on_new_window(move |_url, _new_window| {
                    tauri::webview::NewWindowResponse::Deny
                });

            let app_handle_for_box_new = app.handle().clone();
            let webview_box = WebviewBuilder::new("pane2_box", WebviewUrl::External(tauri::Url::parse("about:blank").unwrap()))
                .initialization_script(init_script_pane2)
                .on_new_window(move |url, _new_window| {
                    let url_str = url.as_str();
                    if let Ok(target_url) = tauri::Url::parse(url_str) {
                        add_pane3_tab(app_handle_for_box_new.clone(), target_url);
                    }
                    tauri::webview::NewWindowResponse::Deny
                });

            let app_handle_for_reearth_new = app.handle().clone();
            let webview_reearth = WebviewBuilder::new("pane2_reearth", WebviewUrl::External(tauri::Url::parse("about:blank").unwrap()))
                .initialization_script(init_script_pane2)
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                .on_new_window(move |url, _new_window| {
                    let url_str = url.as_str();
                    if let Ok(target_url) = tauri::Url::parse(url_str) {
                        add_pane3_tab(app_handle_for_reearth_new.clone(), target_url);
                    }
                    tauri::webview::NewWindowResponse::Deny
                });

            let app_handle_for_google_new = app.handle().clone();
            let webview_google = WebviewBuilder::new("pane2_google", WebviewUrl::External(tauri::Url::parse("https://www.google.com/maps").unwrap()))
                .initialization_script(init_script_pane2)
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                .on_new_window(move |url, _new_window| {
                    let url_str = url.as_str();
                    if let Ok(target_url) = tauri::Url::parse(url_str) {
                        add_pane3_tab(app_handle_for_google_new.clone(), target_url);
                    }
                    tauri::webview::NewWindowResponse::Deny
                });

            let _wv1 = window.add_child(webview1_builder, PhysicalPosition::new(0, 0), PhysicalSize::new(0, 0))?;
            let _wv2 = window.add_child(webview2_builder, PhysicalPosition::new(0, 0), PhysicalSize::new(0, 0))?;
            let _wv_box = window.add_child(webview_box, PhysicalPosition::new(0, 0), PhysicalSize::new(0, 0))?;
            let _wv_reearth = window.add_child(webview_reearth, PhysicalPosition::new(0, 0), PhysicalSize::new(0, 0))?;
            let app_handle_for_googleearth_new = app.handle().clone();
            let webview_googleearth = WebviewBuilder::new("pane2_googleearth", WebviewUrl::External(tauri::Url::parse("https://earth.google.com/web/").unwrap()))
                .initialization_script(init_script_pane2)
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                .on_new_window(move |url, _new_window| {
                    let url_str = url.as_str();
                    if let Ok(target_url) = tauri::Url::parse(url_str) {
                        add_pane3_tab(app_handle_for_googleearth_new.clone(), target_url);
                    }
                    tauri::webview::NewWindowResponse::Deny
                });

            let app_handle_for_yahoo_new = app.handle().clone();
            let webview_yahoo = WebviewBuilder::new("pane2_yahoo", WebviewUrl::External(tauri::Url::parse("https://map.yahoo.co.jp/").unwrap()))
                .initialization_script(init_script_pane2)
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                .on_new_window(move |url, _new_window| {
                    let url_str = url.as_str();
                    if let Ok(target_url) = tauri::Url::parse(url_str) {
                        add_pane3_tab(app_handle_for_yahoo_new.clone(), target_url);
                    }
                    tauri::webview::NewWindowResponse::Deny
                });

            let webview_cesium = WebviewBuilder::new("pane2_cesium", WebviewUrl::App("cesium.html".into()))
                .initialization_script(r#"
                    window.addEventListener('dblclick', function(e) {
                        if (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke) {
                            window.__TAURI_INTERNALS__.invoke('pane_dblclick', { pane: 'pane2_cesium' });
                        } else if (window.__TAURI__ && window.__TAURI__.core) {
                            window.__TAURI__.core.invoke('pane_dblclick', { pane: 'pane2_cesium' });
                        }
                    });
                "#)
                .on_navigation(move |url| {
                    let url_str = url.as_str();
                    if url_str.starts_with("tauri://") || url_str.contains("localhost") || url_str.contains("127.0.0.1") 
                       || url_str.contains("index.html") || url_str.contains("index3.html") 
                       || url_str.contains("cesium.html") {
                        return true;
                    }
                    false
                });

            let _wv_google = window.add_child(webview_google, PhysicalPosition::new(0, 0), PhysicalSize::new(0, 0))?;
            let _wv_googleearth = window.add_child(webview_googleearth, PhysicalPosition::new(0, 0), PhysicalSize::new(0, 0))?;
            let _wv_yahoo = window.add_child(webview_yahoo, PhysicalPosition::new(0, 0), PhysicalSize::new(0, 0))?;
            let _wv_cesium = window.add_child(webview_cesium, PhysicalPosition::new(0, 0), PhysicalSize::new(0, 0))?;
            let _wv3 = window.add_child(webview3_builder, PhysicalPosition::new(0, 0), PhysicalSize::new(0, 0))?;
            
            let state = app.state::<SplitterState>();
            recalculate_webview_bounds(&window, width, height, 0.1, 0.8, "default", &state);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
