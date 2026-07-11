// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;
use tauri::{
    Manager, Position, Rect, Size, WebviewBuilder, WebviewUrl, WindowBuilder,
    PhysicalPosition, PhysicalSize
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
    pane3_current_host: Mutex<Option<String>>,
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
        
        let mut target_str = "pane2";
        if url.contains("earth.google.com") {
            *state.active_pane2.lock().unwrap() = "googleearth".to_string();
            target_str = "pane2_googleearth";
            update_splitter_internal(&app_handle, &state);
        } else if url.contains("google.com/maps") {
            *state.active_pane2.lock().unwrap() = "google".to_string();
            target_str = "pane2_google";
            update_splitter_internal(&app_handle, &state);
        } else if url.contains("box.com") {
            *state.active_pane2.lock().unwrap() = "box".to_string();
            target_str = "pane2_box";
            update_splitter_internal(&app_handle, &state);
        } else if url.contains("reearth.io") {
            *state.active_pane2.lock().unwrap() = "reearth".to_string();
            target_str = "pane2_reearth";
            update_splitter_internal(&app_handle, &state);
        } else if url.contains("map.yahoo.co.jp") {
            *state.active_pane2.lock().unwrap() = "yahoo".to_string();
            target_str = "pane2_yahoo";
            update_splitter_internal(&app_handle, &state);
        } else if url.contains("mapion.co.jp") {
            *state.active_pane2.lock().unwrap() = "mapion".to_string();
            target_str = "pane2_mapion";
            update_splitter_internal(&app_handle, &state);
        } else {
            *state.active_pane2.lock().unwrap() = "default".to_string();
            target_str = "pane2";
            update_splitter_internal(&app_handle, &state);
        }

        if target_str == "pane3" {
            add_pane3_tab(app_handle.clone(), target_url);
        } else {
            if let Some(window) = app_handle.get_window("main") {
                if let Some(wv) = window.get_webview(target_str) {
                    if let Some(h) = host { *state.pane2_current_host.lock().unwrap() = Some(h); }
                    
                    let should_navigate = if let Ok(current_url) = wv.url() {
                        // 現在のURLと異なる場合（位置情報などのパラメータ変更含む）は必ずナビゲートする
                        current_url.as_str() != target_url.as_str() && current_url.as_str() != format!("{}/", target_url.as_str())
                    } else {
                        true
                    };
                    
                    if should_navigate {
                        let _ = wv.navigate(target_url);
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
        let mut target_str = "pane3";
        if url.contains("earth.google.com") {
            *state.active_pane2.lock().unwrap() = "googleearth".to_string();
            target_str = "pane2_googleearth";
            update_splitter_internal(&app_handle, &state);
        } else if url.contains("google.com/maps") {
            *state.active_pane2.lock().unwrap() = "google".to_string();
            target_str = "pane2_google";
            update_splitter_internal(&app_handle, &state);
        } else if url.contains("box.com") {
            *state.active_pane2.lock().unwrap() = "box".to_string();
            target_str = "pane2_box";
            update_splitter_internal(&app_handle, &state);
        } else if url.contains("reearth.io") {
            *state.active_pane2.lock().unwrap() = "reearth".to_string();
            target_str = "pane2_reearth";
            update_splitter_internal(&app_handle, &state);
        } else if url.contains("map.yahoo.co.jp") {
            *state.active_pane2.lock().unwrap() = "yahoo".to_string();
            target_str = "pane2_yahoo";
            update_splitter_internal(&app_handle, &state);
        } else if url.contains("mapion.co.jp") {
            *state.active_pane2.lock().unwrap() = "mapion".to_string();
            target_str = "pane2_mapion";
            update_splitter_internal(&app_handle, &state);
        } else {
            target_str = "pane3";
        }

        if target_str == "pane3" {
            add_pane3_tab(app_handle.clone(), target_url);
        } else {
            if let Some(window) = app_handle.get_window("main") {
                if let Some(wv) = window.get_webview(target_str) {
                    let should_navigate = if let Ok(current_url) = wv.url() {
                        current_url.as_str() != target_url.as_str() && current_url.as_str() != format!("{}/", target_url.as_str())
                    } else {
                        true
                    };
                    
                    if should_navigate {
                        let _ = wv.navigate(target_url);
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
    *state.ratio1.lock().unwrap() = ratio1;
    *state.ratio2.lock().unwrap() = ratio2;
    *state.saved_ratios.lock().unwrap() = None;
    update_splitter_internal(&app_handle, &state);
}

use tauri::Emitter;

#[tauri::command]
fn pane_dblclick(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, SplitterState>,
    pane: String,
) {
    let mut saved = state.saved_ratios.lock().unwrap();
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
            "pane1" => {
                *state.ratio1.lock().unwrap() = 1.0;
                *state.ratio2.lock().unwrap() = 1.0;
            }
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
    let x1 = w * ratio1;
    let x2 = w * ratio2;
    let tab_height = 50.0; // 画面2上部のタブ領域の高さ
    if let Some(base_wv) = window.get_webview("main_webview") {
        let _ = base_wv.set_bounds(Rect {
            position: Position::Physical(PhysicalPosition::new(0, 0)),
            size: Size::Physical(PhysicalSize::new(w as u32, h as u32)),
        });
    }
    if let Some(wv1) = window.get_webview("pane1") {
        let width = (x1 - sh).max(0.0) as u32;
        let _ = wv1.set_bounds(Rect {
            position: Position::Physical(PhysicalPosition::new(0, 0)),
            size: Size::Physical(PhysicalSize::new(width, h as u32)),
        });
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
    let rect_hidden = Rect {
        position: Position::Physical(PhysicalPosition::new(-10000, -10000)),
        size: Size::Physical(PhysicalSize::new(1, 1)),
    };

    let pane2_rect = rect_center;
    let pane2_dedicated_rect = rect_center_dedicated;
    let pane3_rect = rect_right;
    let pane3_dedicated_rect = rect_right_dedicated;

    let update_pane2 = |id: &str, is_active: bool, is_dedicated: bool| {
        if let Some(wv) = window.get_webview(id) {
            if is_active {
                let _ = wv.set_bounds(if is_dedicated { pane2_dedicated_rect } else { pane2_rect });
            } else {
                let _ = wv.set_bounds(rect_hidden);
            }
        }
    };

    // pane2 (ベース画面) は、常に表示しておく（アクティブかどうかに関わらず、背面またはタブ領域として表示する）
    // ただし、完全に非表示にするのではなく、pane2 は常に配置しておくことでタブ部分が見えるようにする
    if let Some(wv2) = window.get_webview("pane2") {
        let _ = wv2.set_bounds(pane2_rect);
    }
    
    // update_pane2("pane2", active_pane2 == "default", false); // 上で常に表示にしたので不要
    update_pane2("pane2_box", active_pane2 == "box", true);
    update_pane2("pane2_reearth", active_pane2 == "reearth", true);
    update_pane2("pane2_google", active_pane2 == "google", true);
    update_pane2("pane2_googleearth", active_pane2 == "googleearth", true);
    update_pane2("pane2_yahoo", active_pane2 == "yahoo", true);

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
    *state.active_pane2.lock().unwrap() = tab;
    update_splitter_internal(&app_handle, &state);
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
        "mapion" => "pane2_mapion",
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

fn main() {
    tauri::Builder::default()
        .manage(SplitterState {
            ratio1: Mutex::new(0.1),
            ratio2: Mutex::new(0.8),
            saved_ratios: Mutex::new(None),
            pane2_current_host: Mutex::new(None),
            pane3_current_host: Mutex::new(None),
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
            get_pane2_url
        ])
        .setup(|app| {
            let window = WindowBuilder::new(app, "main")
                .title("Kasugai 3-Split Viewer")
                .inner_size(1200.0, 800.0)
                .resizable(true)
                .maximized(true)
                .build()?;
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

            let app_handle_for_nav3 = app.handle().clone();
            let app_handle_for_new_window3 = app.handle().clone();
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

            let _wv_google = window.add_child(webview_google, PhysicalPosition::new(0, 0), PhysicalSize::new(0, 0))?;
            let _wv_googleearth = window.add_child(webview_googleearth, PhysicalPosition::new(0, 0), PhysicalSize::new(0, 0))?;
            let _wv_yahoo = window.add_child(webview_yahoo, PhysicalPosition::new(0, 0), PhysicalSize::new(0, 0))?;
            let _wv3 = window.add_child(webview3_builder, PhysicalPosition::new(0, 0), PhysicalSize::new(0, 0))?;
            
            let state = app.state::<SplitterState>();
            recalculate_webview_bounds(&window, width, height, 0.1, 0.8, "default", &state);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
