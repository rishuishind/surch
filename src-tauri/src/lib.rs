// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

#[derive(serde::Serialize)]
struct AppItem {
    name: String,
    exec: String,
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn list_apps() -> Vec<AppItem> {
    vec![
        AppItem {
            name: "Firefox".into(),
            exec: "firefox".into(),
        },
        AppItem {
            name: "VS Code".into(),
            exec: "code".into(),
        },
    ]
}

#[tauri::command]
fn run_app(app: String) -> Result<(), String> {
    use std::process::Command;

    Command::new(app)
        .spawn()
        .map_err(|e| e.to_string())?;

    Ok(())
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, run_app,list_apps])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
