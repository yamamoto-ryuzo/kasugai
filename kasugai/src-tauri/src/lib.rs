use serde::{Deserialize, Serialize};
use tauri::Manager;
use std::collections::HashMap;
use tauri::WebviewWindow;
use std::sync::{Arc, Mutex};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct TabConfig {
    name: String,
    url: String,
}

struct AppState {
    windows: Arc<Mutex<HashMap<String, WebviewWindow>>>,
}

#[tauri::command]
fn get_tabs_config(app: tauri::AppHandle) -> Result<Vec<TabConfig>, String> {
    let resource_path = app
        .path()
        .resolve("config.json", tauri::path::BaseDirectory::Resource)
        .map_err(|e| e.to_string())?;

    let file_content = std::fs::read_to_string(resource_path).map_err(|e| e.to_string())?;

    let configs: Vec<TabConfig> =
        serde_json::from_str(&file_content).map_err(|e| e.to_string())?;

    Ok(configs)
}

#[tauri::command]
async fn create_webview_window(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    label: String,
    url: String,
) -> Result<(), String> {
    let _main_window = app.get_webview_window("main").unwrap();
    let window = tauri::WebviewWindowBuilder::new(&app, &label, tauri::WebviewUrl::External(url.parse().unwrap()))
        .visible(false)
        .build()
        .map_err(|e| e.to_string())?;

    #[cfg(target_os = "windows")]
    window_vibrancy::apply_blur(&window, Some((18, 18, 18, 125)))
      .expect("Unsupported platform! 'apply_blur' is only supported on Windows");

    #[cfg(target_os = "macos")]
    window_vibrancy::apply_vibrancy(&window, NSVisualEffectMaterial::HudWindow)
      .expect("Unsupported platform! 'apply_vibrancy' is only supported on macOS");


    let mut windows = state.windows.lock().unwrap();
    windows.insert(label.clone(), window);
    Ok(())
}

#[tauri::command]
fn show_window(label: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let windows = state.windows.lock().unwrap();
    if let Some(window) = windows.get(&label) {
        window.show().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn hide_window(label: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let windows = state.windows.lock().unwrap();
    if let Some(window) = windows.get(&label) {
        window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            windows: Arc::new(Mutex::new(HashMap::new())),
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_tabs_config,
            create_webview_window,
            show_window,
            hide_window
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
