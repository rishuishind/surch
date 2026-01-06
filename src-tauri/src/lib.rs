// Learn more about Tauri commands at https://tauri.app/develop/rust/

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use rayon::prelude::*;
use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::collections::{HashMap, HashSet};
use std::path::Path;

#[derive(serde::Serialize)]
struct AppItem {
    name: String,
    exec: String,
}

#[derive(serde::Serialize, Clone)]
struct SearchResult {
    name: String,
    path: String,
    kind: String,
    score: i16,
    icon: Option<String>,
}

#[derive(Debug, Clone)]
struct DesktopEntry {
    exec: String,
    icon: Option<String>,
}

fn infer_kind(path: &str) -> String {
    let p= std::path::Path::new(path);
    match p.extension().and_then(|s| s.to_str()).map(|s| s.to_lowercase()){
        Some(ext) if ext == "pdf" => "pdf".into(),
        Some(ext) if ext == "docx" || ext == "doc" || ext == "odt" => "document".into(),
        Some(ext) if ext == "txt" || ext == "md" => "text".into(),
        Some(ext) if ext == "png" || ext == "jpg" || ext == "jpeg" || ext == "webp" => "image".into(),
        Some(ext) if ext == "desktop" => "desktop".into(),
        Some(ext) if ext == "sh" || ext == "py" || ext == "bin" || ext == "exe" => "script".into(),
        Some(_) => "file".into(),
        None => "file".into(),
    }
}

/// Parse a .desktop file and extract Name, Exec, and Icon fields
fn parse_desktop_file(path: &Path) -> Option<DesktopEntry> {
    let content = fs::read_to_string(path).ok()?;
    let mut name = None;
    let mut exec = None;
    let mut icon = None;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("Name=") && name.is_none() {
            name = Some(line[5..].to_string());
        } else if line.starts_with("Exec=") {
            let exec_line = line[5..].to_string();
            // Extract the actual executable name (first word before any arguments)
            let executable = exec_line
                .split_whitespace()
                .next()
                .unwrap_or(&exec_line)
                .to_string();
            exec = Some(executable);
        } else if line.starts_with("Icon=") {
            icon = Some(line[5..].to_string());
        }
    }

    Some(DesktopEntry {
        exec: exec?,
        icon,
    })
}

/// Resolve an icon name to an actual file path
fn resolve_icon_path(icon_name: &str) -> Option<String> {
    // If it's already a full path, return it
    if icon_name.starts_with('/') && Path::new(icon_name).exists() {
        return Some(icon_name.to_string());
    }
    
    // Icon theme directories to search
    let icon_dirs = vec![
        "/usr/share/pixmaps",
        "/usr/share/icons/hicolor",
        "/usr/share/icons",
    ];
    
    // Add user's local icon directory
    let mut search_dirs = Vec::new();
    if let Ok(home) = env::var("HOME") {
        search_dirs.push(format!("{}/.local/share/icons", home));
        search_dirs.push(format!("{}/.icons", home));
    }
    search_dirs.extend(icon_dirs.iter().map(|s| s.to_string()));
    
    // Common icon sizes to search for (in order of preference)
    let sizes = vec!["128x128", "256x256", "scalable", "64x64", "48x48", "32x32"];
    let extensions = vec!["png", "svg", "xpm"];
    
    // First, try exact matches in pixmaps
    for dir in &search_dirs {
        for ext in &extensions {
            let path = format!("{}/{}.{}", dir, icon_name, ext);
            if Path::new(&path).exists() {
                return Some(path);
            }
        }
    }
    
    // Then search in hicolor theme with different sizes
    for dir in &search_dirs {
        for size in &sizes {
            for ext in &extensions {
                // Try apps category
                let path = format!("{}/{}/apps/{}.{}", dir, size, icon_name, ext);
                if Path::new(&path).exists() {
                    return Some(path);
                }
                
                // Try without category
                let path = format!("{}/{}/{}.{}", dir, size, icon_name, ext);
                if Path::new(&path).exists() {
                    return Some(path);
                }
            }
        }
    }
    
    None
}

/// Find and parse all desktop files from standard locations
fn find_desktop_files() -> HashMap<String, DesktopEntry> {
    let mut desktop_map = HashMap::new();
    
    // Desktop file locations in order of priority
    let desktop_dirs = vec![
        "/var/lib/snapd/desktop/applications",
        "/usr/share/applications",
    ];
    
    // Add user's local applications directory
    if let Ok(home) = env::var("HOME") {
        let user_apps = format!("{}/.local/share/applications", home);
        for dir in [user_apps.as_str()].iter().chain(desktop_dirs.iter()) {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("desktop") {
                        if let Some(mut desktop_entry) = parse_desktop_file(&path) {
                            // Resolve icon name to actual path if needed
                            if let Some(ref icon) = desktop_entry.icon {
                                if !icon.starts_with('/') {
                                    desktop_entry.icon = resolve_icon_path(icon);
                                }
                            }
                            
                            // Extract executable name from the exec path
                            let exec_name = Path::new(&desktop_entry.exec)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or(&desktop_entry.exec)
                                .to_string();
                            
                            // Also try to match by desktop filename (e.g., cursor.desktop -> cursor)
                            let desktop_filename = path
                                .file_stem()
                                .and_then(|n| n.to_str())
                                .map(|s| s.to_string());
                            
                            // Add entry for both exec name and desktop filename
                            desktop_map.entry(exec_name.clone()).or_insert(desktop_entry.clone());
                            if let Some(df_name) = desktop_filename {
                                if df_name != exec_name {
                                    desktop_map.entry(df_name).or_insert(desktop_entry);
                                }
                            }
                        }
                    }
                }
            }
        }
    } else {
        for dir in desktop_dirs.iter() {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("desktop") {
                        if let Some(mut desktop_entry) = parse_desktop_file(&path) {
                            // Resolve icon name to actual path if needed
                            if let Some(ref icon) = desktop_entry.icon {
                                if !icon.starts_with('/') {
                                    desktop_entry.icon = resolve_icon_path(icon);
                                }
                            }
                            
                            let exec_name = Path::new(&desktop_entry.exec)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or(&desktop_entry.exec)
                                .to_string();
                            
                            let desktop_filename = path
                                .file_stem()
                                .and_then(|n| n.to_str())
                                .map(|s| s.to_string());
                            
                            desktop_map.entry(exec_name.clone()).or_insert(desktop_entry.clone());
                            if let Some(df_name) = desktop_filename {
                                if df_name != exec_name {
                                    desktop_map.entry(df_name).or_insert(desktop_entry);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    desktop_map
}


#[tauri::command]
fn search_path_executables(query: String) -> Vec<SearchResult> {
    let query = query.trim();
    if query.is_empty() {
        return vec![];
    }

    let matcher = SkimMatcherV2::default();
    
    // Load desktop file information
    let desktop_files = find_desktop_files();

    let path_var = env::var("PATH").unwrap_or_default();
    let mut seen_names = HashSet::new();
    let mut candidates: Vec<(String, String)> = Vec::new();

    for dir in path_var.split(':') {
        let Ok(entries) = fs::read_dir(dir) else { continue };

        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(meta) = entry.metadata() else { continue };

            // Must be a file and executable
            if meta.permissions().mode() & 0o111 != 0 {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    // Deduplicate: only add if we haven't seen this name before
                    if seen_names.insert(name.to_string()) {
                        candidates.push((name.to_string(), path.display().to_string()));
                    }
                }
            }
        }
    }

    let mut results: Vec<SearchResult> = candidates
        .par_iter()
        .filter_map(|(name, path)| {
            matcher
                .fuzzy_match(name, query)
                .map(|score| {
                    // Try to get icon from desktop file and convert to base64
                    let icon = desktop_files
                        .get(name)
                        .and_then(|entry| entry.icon.clone())
                        .and_then(|icon_path| {
                            // Convert icon path to base64 data URL
                            get_icon_data(icon_path).ok()
                        });
                    
                    SearchResult {
                        name: name.clone(),
                        path: path.clone(),
                        kind: "app".to_string(),
                        score: score as i16,
                        icon,
                    }
                })
        })
        .collect();

    results.par_sort_unstable_by(|a, b| b.score.cmp(&a.score));
    results.truncate(50);
    results
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

#[tauri::command]
fn open_file(path:String) -> Result<(),String>{
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open")
        .arg(&path)
        .spawn();

    #[cfg(target_os = "linux")]
    let result = std::process::Command::new("xdg-open")
        .arg(&path)
        .spawn();

    result.map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn start(file_type:String,path:String) -> (){
    match file_type.as_str() {
        "app" | "file" => {
            let _ = run_app(path);
        }
        "pdf" | "document" | "text" | "image" | "script" => {
            let _ = open_file(path);
        }
        _ => {
            let _ = open_file(path);
        }
    }
} 

#[tauri::command]
fn get_icon_data(icon_path: String) -> Result<String, String> {
    use std::io::Read;
    
    // Read the icon file
    let mut file = fs::File::open(&icon_path).map_err(|e| e.to_string())?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
    
    // Convert to base64
    let base64_data = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &buffer);
    
    // Determine MIME type based on file extension
    let mime_type = if icon_path.ends_with(".png") {
        "image/png"
    } else if icon_path.ends_with(".svg") {
        "image/svg+xml"
    } else if icon_path.ends_with(".jpg") || icon_path.ends_with(".jpeg") {
        "image/jpeg"
    } else {
        "image/png" // default
    };
    
    Ok(format!("data:{};base64,{}", mime_type, base64_data))
}
 

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![run_app,list_apps,search_path_executables,open_file,start,get_icon_data])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
