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
    pane2_current_host: Mutex<Option<String>>,
    pane3_current_host: Mutex<Option<String>>,
    pane_swapped: Mutex<bool>,
    reearth_email: Mutex<Option<String>>,
    box_email: Mutex<Option<String>>,
    active_pane2: Mutex<String>,
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
    target: String, // "pane2" or "pane3"
    url: String,
    creds: AutologinCreds,
) {
    let swapped = *state.pane_swapped.lock().unwrap();
    let real_target = if target == "pane2" {
        if !swapped {
            *state.active_pane2.lock().unwrap() = "box".to_string();
            update_splitter_internal(&app_handle, &state);
            "pane2_box"
        } else { "pane3" }
    } else if target == "pane3" {
        if !swapped { "pane3" } else {
            *state.active_pane2.lock().unwrap() = "box".to_string();
            update_splitter_internal(&app_handle, &state);
            "pane2_box"
        }
    } else {
        &target
    };

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
    target: String,
    url: String,
    creds: AutologinCreds,
) {
    let swapped = *state.pane_swapped.lock().unwrap();
    let real_target = if target == "pane2" {
        if !swapped {
            *state.active_pane2.lock().unwrap() = "reearth".to_string();
            update_splitter_internal(&app_handle, &state);
            "pane2_reearth"
        } else { "pane3" }
    } else if target == "pane3" {
        if !swapped { "pane3" } else {
            *state.active_pane2.lock().unwrap() = "reearth".to_string();
            update_splitter_internal(&app_handle, &state);
            "pane2_reearth"
        }
    } else {
        &target
    };

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

// 通常のナビゲーション（Google Map等）
#[tauri::command]
fn open_in_pane2(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, SplitterState>,
    url: String,
) {
    let swapped = *state.pane_swapped.lock().unwrap();
    if let Ok(target_url) = tauri::Url::parse(&url) {
        let host = target_url.host_str().map(|h| h.to_string());
        
        let mut target_str = "pane2";
        if !swapped {
            if url.contains("google.com/maps") {
                *state.active_pane2.lock().unwrap() = "google".to_string();
                target_str = "pane2_google";
            } else if url.contains("box.com") {
                *state.active_pane2.lock().unwrap() = "box".to_string();
                target_str = "pane2_box";
            } else if url.contains("reearth.io") {
                *state.active_pane2.lock().unwrap() = "reearth".to_string();
                target_str = "pane2_reearth";
            } else {
                *state.active_pane2.lock().unwrap() = "default".to_string();
                target_str = "pane2";
            }
            update_splitter_internal(&app_handle, &state);
        } else {
            target_str = "pane3";
        }

        if let Some(window) = app_handle.get_window("main") {
            if let Some(wv) = window.get_webview(target_str) {
                if !swapped { if let Some(h) = host { *state.pane2_current_host.lock().unwrap() = Some(h); } }
                else { if let Some(h) = host { *state.pane3_current_host.lock().unwrap() = Some(h); } }
                
                let is_dedicated = target_str.starts_with("pane2_");
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
    let swapped = *state.pane_swapped.lock().unwrap();
    if let Ok(target_url) = tauri::Url::parse(&url) {
        let mut target_str = "pane3";
        if swapped {
            if url.contains("google.com/maps") {
                *state.active_pane2.lock().unwrap() = "google".to_string();
                target_str = "pane2_google";
            } else if url.contains("box.com") {
                *state.active_pane2.lock().unwrap() = "box".to_string();
                target_str = "pane2_box";
            } else if url.contains("reearth.io") {
                *state.active_pane2.lock().unwrap() = "reearth".to_string();
                target_str = "pane2_reearth";
            } else {
                *state.active_pane2.lock().unwrap() = "default".to_string();
                target_str = "pane2";
            }
            update_splitter_internal(&app_handle, &state);
        }

        if let Some(window) = app_handle.get_window("main") {
            if let Some(wv) = window.get_webview(target_str) {
                let is_dedicated = target_str.starts_with("pane2_");
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
    update_splitter_internal(&app_handle, &state);
}

fn update_splitter_internal(app_handle: &tauri::AppHandle, state: &tauri::State<'_, SplitterState>) {
    if let Some(window) = app_handle.get_window("main") {
        if let Ok(size) = window.inner_size() {
            let w = size.width as f64;
            let h = size.height as f64;
            let r1 = *state.ratio1.lock().unwrap();
            let r2 = *state.ratio2.lock().unwrap();
            let swapped = *state.pane_swapped.lock().unwrap();
            let active = state.active_pane2.lock().unwrap().clone();
            recalculate_webview_bounds(&window, w, h, r1, r2, swapped, &active);
        }
    }
}

fn recalculate_webview_bounds(window: &tauri::Window, w: f64, h: f64, ratio1: f64, ratio2: f64, swapped: bool, active_pane2: &str) {
    let splitter_width = 8.0;
    let sh = splitter_width / 2.0;
    let x1 = w * ratio1;
    let x2 = w * ratio2;
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
    let rect_right = Rect {
        position: Position::Physical(PhysicalPosition::new((x2 + sh) as i32, 0)),
        size: Size::Physical(PhysicalSize::new((w - (x2 + sh)).max(0.0) as u32, h as u32)),
    };
    let rect_hidden = Rect {
        position: Position::Physical(PhysicalPosition::new(-10000, -10000)),
        size: Size::Physical(PhysicalSize::new(1, 1)),
    };

    let pane2_rect = if !swapped { rect_center } else { rect_right };
    let pane3_rect = if !swapped { rect_right } else { rect_center };

    let update_pane2 = |id: &str, is_active: bool| {
        if let Some(wv) = window.get_webview(id) {
            if is_active {
                let _ = wv.set_bounds(pane2_rect);
            } else {
                let _ = wv.set_bounds(rect_hidden);
            }
        }
    };

    update_pane2("pane2", active_pane2 == "default");
    update_pane2("pane2_box", active_pane2 == "box");
    update_pane2("pane2_reearth", active_pane2 == "reearth");
    update_pane2("pane2_google", active_pane2 == "google");

    if let Some(wv3) = window.get_webview("pane3") {
        let _ = wv3.set_bounds(pane3_rect);
    }
}

#[tauri::command]
fn set_center(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, SplitterState>,
    center: String,
) {
    {
        let mut swapped = state.pane_swapped.lock().unwrap();
        if center == "toggle" {
            *swapped = !*swapped;
        } else {
            *swapped = match center.as_str() {
                "pane2" => false,
                "pane3" => true,
                _ => *swapped,
            };
        }
    }
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
            pane2_current_host: Mutex::new(None),
            pane3_current_host: Mutex::new(None),
            pane_swapped: Mutex::new(false),
            reearth_email: Mutex::new(None),
            box_email: Mutex::new(None),
            active_pane2: Mutex::new("default".to_string()),
        })
        .invoke_handler(tauri::generate_handler![
            get_system_info,
            update_splitter,
            open_in_pane2,
            open_in_pane3,
            open_box_in_pane,
            open_reearth_in_pane,
            set_center,
            save_credential,
            get_credential,
            delete_credential,
            prefetch_basic_auth,
            type_credentials,
            preload_webview
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
                    if url_str.starts_with("tauri://") || url_str.contains("localhost") || url_str.contains("127.0.0.1") 
                       || url_str.contains("index2.html") || url_str.contains("index3.html") 
                       || url_str.contains("account.box.com") || url_str.contains("app.box.com") || url_str.contains("reearth.io") {
                        return true;
                    }
                    let state = app_handle_for_nav2.state::<SplitterState>();
                    let swapped = *state.pane_swapped.lock().unwrap();
                    if !swapped {
                        let allowed_host_opt = state.pane2_current_host.lock().unwrap().clone();
                        if let Some(target_host) = url.host_str() {
                            if let Some(allowed_host) = allowed_host_opt {
                                if target_host == allowed_host || target_host.ends_with(&format!(".{}", allowed_host)) {
                                    return true;
                                }
                            }
                        }
                        if let Some(window) = app_handle_for_nav2.get_window("main") {
                            if let Some(wv3) = window.get_webview("pane3") {
                                if let Ok(target_url) = tauri::Url::parse(url_str) {
                                    let _ = wv3.navigate(target_url);
                                    return false;
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
                            if let Some(wv3) = window.get_webview("pane3") {
                                if let Ok(target_url) = tauri::Url::parse(url_str) {
                                    let _ = wv3.navigate(target_url);
                                }
                            }
                        }
                    }
                    tauri::webview::NewWindowResponse::Deny
                });

            let app_handle_for_nav3 = app.handle().clone();
            let app_handle_for_new_window3 = app.handle().clone();
            let webview3_builder = WebviewBuilder::new("pane3", WebviewUrl::App("index3.html".into()))
                .on_navigation(move |url| {
                    let url_str = url.as_str();
                    if url_str.starts_with("tauri://") || url_str.contains("localhost") || url_str.contains("127.0.0.1") 
                       || url_str.contains("index2.html") || url_str.contains("index3.html") 
                       || url_str.contains("account.box.com") || url_str.contains("app.box.com") || url_str.contains("reearth.io") {
                        return true;
                    }
                    let state = app_handle_for_nav3.state::<SplitterState>();
                    let swapped = *state.pane_swapped.lock().unwrap();
                    if swapped {
                        let allowed_host_opt = state.pane3_current_host.lock().unwrap().clone();
                        if let Some(target_host) = url.host_str() {
                            if let Some(allowed_host) = allowed_host_opt {
                                if target_host == allowed_host || target_host.ends_with(&format!(".{}", allowed_host)) {
                                    return true;
                                }
                            }
                        }
                        if let Some(window) = app_handle_for_nav3.get_window("main") {
                            if let Some(wv2) = window.get_webview("pane2") {
                                if let Ok(target_url) = tauri::Url::parse(url_str) {
                                    let _ = wv2.navigate(target_url);
                                    return false;
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
                            if let Some(wv2) = window.get_webview("pane2") {
                                if let Ok(target_url) = tauri::Url::parse(url_str) {
                                    let _ = wv2.navigate(target_url);
                                }
                            }
                        }
                    }
                    tauri::webview::NewWindowResponse::Deny
                });

            let webview_box = WebviewBuilder::new("pane2_box", WebviewUrl::External(tauri::Url::parse("about:blank").unwrap()));
            let webview_reearth = WebviewBuilder::new("pane2_reearth", WebviewUrl::External(tauri::Url::parse("about:blank").unwrap()));
            let webview_google = WebviewBuilder::new("pane2_google", WebviewUrl::External(tauri::Url::parse("https://www.google.com/maps").unwrap()));

            let _wv1 = window.add_child(webview1_builder, PhysicalPosition::new(0, 0), PhysicalSize::new(0, 0))?;
            let _wv2 = window.add_child(webview2_builder, PhysicalPosition::new(0, 0), PhysicalSize::new(0, 0))?;
            let _wv_box = window.add_child(webview_box, PhysicalPosition::new(0, 0), PhysicalSize::new(0, 0))?;
            let _wv_reearth = window.add_child(webview_reearth, PhysicalPosition::new(0, 0), PhysicalSize::new(0, 0))?;
            let _wv_google = window.add_child(webview_google, PhysicalPosition::new(0, 0), PhysicalSize::new(0, 0))?;
            let _wv3 = window.add_child(webview3_builder, PhysicalPosition::new(0, 0), PhysicalSize::new(0, 0))?;
            
            recalculate_webview_bounds(&window, width, height, 0.1, 0.8, false, "default");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
