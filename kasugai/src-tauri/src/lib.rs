use serde::{Deserialize, Serialize};
use tauri::Manager;

#[derive(Debug, Serialize, Deserialize)]
struct TabConfig {
    name: String,
    url: String,
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![get_tabs_config])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
