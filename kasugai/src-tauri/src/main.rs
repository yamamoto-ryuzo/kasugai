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
    reearth_email: Mutex<Option<String>>, // 現在設定されているRe:Earthメールアドレス
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

// 究極の「OSネイティブ・キーボードエミュレーション」用スクリプト
const AUTOLOGIN_SCRIPT: &str = r#"
(function() {
    console.log("[Kasugai Init] Native Emulation Agent active.");
    
    let credsCache = null;
    let hasFilled = false;

    // 1秒ごとにパスワード欄を探し続ける（無限ループエージェント）
    const masterTimer = setInterval(() => {
        // 設定画面（ローカル環境）では絶対に暴走しないようにする安全装置
        const url = window.location.href;
        if (url.includes("localhost") || url.includes("tauri://") || url.includes("index")) return;

        if (hasFilled) return; // 一度実行したら休止

        let passInput = document.querySelector('input[type="password"]') ||
                        document.querySelector('input[name*="pass"]') ||
                        document.querySelector('input[id*="pass"]');
                        
        let emailInput = document.querySelector('input[type="email"]') ||
                         document.querySelector('input[name*="user"]') ||
                         document.querySelector('input[name*="email"]') ||
                         document.querySelector('input[id*="username"]') ||
                         document.querySelector('input[id*="email"]');
                         
        if (passInput && emailInput) {
            // 枠を発見！Rustバックエンドからパスワードを要求する
            if (!credsCache && window.__TAURI__ && window.__TAURI__.core) {
                window.__TAURI__.core.invoke('get_reearth_creds_for_autologin').then((creds) => {
                    if (creds && creds.email && creds.password) {
                        credsCache = creds;
                        triggerNativeEmulation(emailInput);
                    } else {
                        console.warn("[Kasugai Init] 資格情報が空で返されました。設定を確認してください。");
                    }
                }).catch(e => console.warn("[Kasugai Init] Failed to get creds:", e));
            } else if (credsCache) {
                triggerNativeEmulation(emailInput);
            }
        }
    }, 1000);

    function triggerNativeEmulation(emailInput) {
        if (hasFilled || !credsCache) return;
        
        console.log("[Kasugai Init] Form found! Triggering native keyboard emulation...");
        hasFilled = true;
        
        // メールアドレス欄を空にしてフォーカスを当てる（カーソルを置く）
        emailInput.value = ""; 
        emailInput.focus();
        emailInput.click();

        // 少しだけフォーカスが安定するのを待ってからRustに物理打鍵を命令する
        setTimeout(() => {
            window.__TAURI__.core.invoke('type_credentials', {
                email: credsCache.email,
                password: credsCache.password
            });
        }, 500);

        // 別のログイン画面（多段認証など）に備えて10秒後に再度エージェントを再起動
        setTimeout(() => { hasFilled = false; }, 10000);
    }
})();
"#;

// WebViewが自律的にログイン情報を取得するためのTauriコマンド
#[tauri::command]
fn get_reearth_creds_for_autologin(
    state: tauri::State<'_, SplitterState>,
) -> Result<Option<AutologinCreds>, String> {
    let email_opt = state.reearth_email.lock().unwrap().clone();
    if let Some(email) = email_opt {
        if let Ok(entry) = Entry::new("Kasugai_Reearth", &email) {
            if let Ok(password) = entry.get_password() {
                return Ok(Some(AutologinCreds { email, password }));
            }
        }
    }
    Ok(None)
}

// OSネイティブ・物理キーボードタイピングエミュレーションコマンド
#[tauri::command]
fn type_credentials(email: String, password: String) {
    // KASUGAIによる「物理キーボード打鍵エミュレーション」
    thread::spawn(move || {
        // Rust側でも念のため500ms待機し、確実にWebビューのフォーカスがアクティブになるのを待つ
        thread::sleep(Duration::from_millis(500));
        
        let mut enigo = Enigo::new();
        
        // 1. メールアドレスを物理的にタイピング
        enigo.key_sequence(&email);
        thread::sleep(Duration::from_millis(200));
        
        // 2. Tabキーを押してパスワード欄へフォーカス移動
        enigo.key_click(Key::Tab);
        thread::sleep(Duration::from_millis(200));
        
        // 3. パスワードを物理的にタイピング
        enigo.key_sequence(&password);
        thread::sleep(Duration::from_millis(200));
        
        // 4. Enterキーを押してログインを実行
        enigo.key_click(Key::Return);
        
        println!("[Kasugai Enigo] Native typing sequence completed.");
    });
}

// 事前に同じWebviewインスタンスのコンテキストでBasic認証をfetchし、認証情報キャッシュを記憶させてから遷移するヘルパー
fn navigate_with_basic_auth(wv: &tauri::Webview<tauri::Wry>, url: &str, creds: &AutologinCreds) {
    let script = format!(
        r#"
        (async function() {{
            try {{
                console.log("[Kasugai] Starting inline Basic Auth fetch for: {}");
                const basicAuthHeader = "Basic " + btoa(unescape(encodeURIComponent("{}:{}")));
                await fetch("{}", {{
                    method: "GET",
                    headers: {{
                        "Authorization": basicAuthHeader
                    }},
                    mode: 'no-cors'
                }});
                console.log("[Kasugai] Basic認証情報の事前キャッシュフェッチに成功しました。");
            }} catch (fetchErr) {{
                console.warn("[Kasugai] 事前キャッシュフェッチでエラー:", fetchErr);
            }} finally {{
                window.location.href = "{}";
            }}
        }})();
        "#,
        url.replace('"', "\\\""),
        creds.email.replace('"', "\\\""),
        creds.password.replace('"', "\\\""),
        url.replace('"', "\\\""),
        url.replace('"', "\\\"")
    );
    let _ = wv.eval(&script);
}

// 画面1(左)から送信されたURLを中央のWebviewで開く（物理的位置に追従）
#[tauri::command]
fn open_in_pane2(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, SplitterState>,
    url: String,
    autologin: Option<AutologinCreds>,
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
                    let is_reearth = url.contains("reearth") || url.contains("visualizer");
                    if is_reearth && autologin.is_some() {
                        let creds = autologin.clone().unwrap();
                        navigate_with_basic_auth(&wv2, &url, &creds);

                        // 1.5秒待機してBasic認証ダイアログが出現した直後に無条件で物理タイピングを実行
                        thread::spawn(move || {
                            thread::sleep(Duration::from_millis(1500));
                            
                            let mut enigo = Enigo::new();
                            enigo.key_sequence(&creds.email);
                            thread::sleep(Duration::from_millis(100));
                            enigo.key_click(Key::Tab);
                            thread::sleep(Duration::from_millis(100));
                            enigo.key_sequence(&creds.password);
                            thread::sleep(Duration::from_millis(100));
                            enigo.key_click(Key::Return);
                            
                            println!("[Kasugai Enigo] Basic Auth dialog typing completed in pane2.");
                        });
                    } else {
                        let _ = wv2.navigate(target_url);
                    }
                    let _ = wv2.set_focus();
                }
            } else {
                // スワップ時：pane3が中央
                if let Some(h) = host {
                    *state.pane3_current_host.lock().unwrap() = Some(h);
                }
                if let Some(wv3) = window.get_webview("pane3") {
                    let is_reearth = url.contains("reearth") || url.contains("visualizer");
                    if is_reearth && autologin.is_some() {
                        navigate_with_basic_auth(&wv3, &url, autologin.as_ref().unwrap());
                    } else {
                        let _ = wv3.navigate(target_url);
                    }
                    let _ = wv3.set_focus();
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
    autologin: Option<AutologinCreds>,
) {
    let swapped = *state.pane_swapped.lock().unwrap();
    if let Some(window) = app_handle.get_window("main") {
        if !swapped {
            // 通常時：pane3が右
            if let Some(wv3) = window.get_webview("pane3") {
                if let Ok(target_url) = tauri::Url::parse(&url) {
                    let is_reearth = url.contains("reearth") || url.contains("visualizer");
                    let _ = wv3.navigate(target_url);
                    let _ = wv3.set_focus();
                    
                    if is_reearth && autologin.is_some() {
                        let creds = autologin.unwrap();
                        // 1.5秒待機してBasic認証ダイアログが出現した直後に無条件で物理タイピングを実行
                        thread::spawn(move || {
                            thread::sleep(Duration::from_millis(1500));
                            
                            let mut enigo = Enigo::new();
                            // IDをタイピング
                            enigo.key_sequence(&creds.email);
                            thread::sleep(Duration::from_millis(100));
                            // Tabでパスワード欄へ
                            enigo.key_click(Key::Tab);
                            thread::sleep(Duration::from_millis(100));
                            // パスワードをタイピング
                            enigo.key_sequence(&creds.password);
                            thread::sleep(Duration::from_millis(100));
                            // Enterでログイン
                            enigo.key_click(Key::Return);
                            
                            println!("[Kasugai Enigo] Basic Auth dialog typing completed.");
                        });
                    }
                }
            }
        } else {
            // スワップ時：pane2が右
            if let Some(wv2) = window.get_webview("pane2") {
                if let Ok(target_url) = tauri::Url::parse(&url) {
                    let is_reearth = url.contains("reearth") || url.contains("visualizer");
                    let _ = wv2.navigate(target_url);
                    let _ = wv2.set_focus();
                    
                    if is_reearth && autologin.is_some() {
                        let creds = autologin.unwrap();
                        // 1.5秒待機してBasic認証ダイアログが出現した直後に無条件で物理タイピングを実行
                        thread::spawn(move || {
                            thread::sleep(Duration::from_millis(1500));
                            
                            let mut enigo = Enigo::new();
                            enigo.key_sequence(&creds.email);
                            thread::sleep(Duration::from_millis(100));
                            enigo.key_click(Key::Tab);
                            thread::sleep(Duration::from_millis(100));
                            enigo.key_sequence(&creds.password);
                            thread::sleep(Duration::from_millis(100));
                            enigo.key_click(Key::Return);
                            
                            println!("[Kasugai Enigo] Basic Auth dialog typing completed.");
                        });
                    }
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

// ウィンドウのサイズ、比率に基づいてWebview의 境界を再設定するヘルパー
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

// 起動時に裏側(pane2/pane3)で事前にBasic認証のfetchを実行し、キャッシュを記憶させるコマンド
#[tauri::command]
fn prefetch_basic_auth(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, SplitterState>,
    url: String,
    creds: AutologinCreds,
) {
    // プリフェッチ時にメールアドレスをグローバルステートにキャッシュして初期化スクリプトが使えるようにする
    *state.reearth_email.lock().unwrap() = Some(creds.email.clone());

    if let Some(window) = app_handle.get_window("main") {
        // pane2でプリフェッチ
        if let Some(wv2) = window.get_webview("pane2") {
            let script = format!(
                r#"
                (async function() {{
                    try {{
                        console.log("[Kasugai] Pre-fetching Basic Auth for pane2");
                        const basicAuthHeader = "Basic " + btoa(unescape(encodeURIComponent("{}:{}")));
                        await fetch("{}", {{
                            method: "GET",
                            headers: {{
                                "Authorization": basicAuthHeader
                            }},
                            mode: 'no-cors'
                        }});
                        console.log("[Kasugai] pane2 Basic認証事前キャッシュ完了");
                    }} catch (e) {{
                        console.error("[Kasugai] pane2 Basic認証事前キャッシュエラー:", e);
                    }}
                }})();
                "#,
                creds.email.replace('"', "\\\""),
                creds.password.replace('"', "\\\""),
                url.replace('"', "\\\"")
            );
            let _ = wv2.eval(&script);
        }
        // pane3でプリフェッチ
        if let Some(wv3) = window.get_webview("pane3") {
            let script = format!(
                r#"
                (async function() {{
                    try {{
                        console.log("[Kasugai] Pre-fetching Basic Auth for pane3");
                        const basicAuthHeader = "Basic " + btoa(unescape(encodeURIComponent("{}:{}")));
                        await fetch("{}", {{
                            method: "GET",
                            headers: {{
                                "Authorization": basicAuthHeader
                            }},
                            mode: 'no-cors'
                        }});
                        console.log("[Kasugai] pane3 Basic認証事前キャッシュ完了");
                    }} catch (e) {{
                        console.error("[Kasugai] pane3 Basic認証事前キャッシュエラー:", e);
                    }}
                }})();
                "#,
                creds.email.replace('"', "\\\""),
                creds.password.replace('"', "\\\""),
                url.replace('"', "\\\"")
            );
            let _ = wv3.eval(&script);
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
        })
        .invoke_handler(tauri::generate_handler![
            get_system_info,
            update_splitter,
            open_in_pane2,
            open_in_pane3,
            set_center,
            save_credential,
            get_credential,
            delete_credential,
            prefetch_basic_auth,
            get_reearth_creds_for_autologin,
            type_credentials // 追加
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
                .initialization_script(AUTOLOGIN_SCRIPT) // 最強の自動ログインスクリプトを登録
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
                        }
                    }
                    
                    tauri::webview::NewWindowResponse::Deny
                });

            // pane3 (通常時は右、swapped時は接続時に同期) の挙動も役割交代に対応
            let app_handle_for_nav3 = app.handle().clone();
            let app_handle_for_new_window3 = app.handle().clone();
            let webview3_builder = WebviewBuilder::new("pane3", WebviewUrl::App("index3.html".into()))
                .initialization_script(AUTOLOGIN_SCRIPT) // 最強の自動ログインスクリプトを登録
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
                match event {
                    tauri::WindowEvent::Resized(new_size) => {
                        let w = new_size.width as f64;
                        let h = new_size.height as f64;

                        // Stateから最新 of 比率を取得して再配置
                        let state = app_handle.state::<SplitterState>();
                        let r1 = *state.ratio1.lock().unwrap();
                        let r2 = *state.ratio2.lock().unwrap();

                        let swapped_now = *state.pane_swapped.lock().unwrap();
                        recalculate_webview_bounds(&window_clone, w, h, r1, r2, swapped_now);
                    }
                    _ => {}
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
