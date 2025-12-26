// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

use walkdir::WalkDir;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use rayon::prelude::*;

#[derive(serde::Serialize)]
struct AppItem {
    name: String,
    exec: String,
}

#[derive(serde::Serialize)]
struct SearchResult {
    name: String,
    path: String,
    kind: String,
    score: i64
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

#[tauri::command]
fn search_file(query:String) -> Vec<SearchResult>{
    let trimmed_query = query.trim();
    if trimmed_query.len() < 1 {
        return Vec::new();
    }

    let mut search_dirs:Vec<String> = Vec::new();
    if let Ok(home) = std::env::var("HOME"){
        search_dirs.push(home);
    }
    search_dirs.push("/usr/share/applications".into());
    search_dirs.push("/usr/bin".into());
    search_dirs.push("/snap/bin".into());
    search_dirs.push("/usr/local".into());
    if let Ok(home) = std::env::var("HOME"){
        search_dirs.push(format!("{}/Desktop",home));
        search_dirs.push(format!("{}/Downloads",home));
        search_dirs.push(format!("{}/Documents",home));
    }

    let mut candidates:Vec<(String,String)> = Vec::new();

   for dir in search_dirs {
        let walker = WalkDir::new(&dir).max_depth(6).into_iter();
        for entry in walker.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() {
                if let Some(fname_os) = path.file_name() {
                    if let Some(fname) = fname_os.to_str() {
                        candidates.push((fname.to_string(), path.display().to_string()));
                    }
                }
            }
        }
    }

    let matcher = SkimMatcherV2::default();
    let mut results: Vec<SearchResult> = candidates
        .par_iter()
        .filter_map(|(name, path)| {
            matcher.fuzzy_match(&name, trimmed_query).map(|score| SearchResult {
                name: name.clone(),
                path: path.clone(),
                kind: infer_kind(path),
                score: score as i64,
            })
        })
        .collect();
    results.par_sort_unstable_by(|a, b| b.score.cmp(&a.score));

    const MAX_RESULTS: usize = 100;
    if results.len() > MAX_RESULTS {
        results.truncate(MAX_RESULTS);
    }

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
    println!("File type received: {}, path: {}", file_type, path);
    match file_type.as_str() {
        "app" | "file" => {
            run_app(path);
        }
        "pdf" | "document" | "text" | "image" | "script" => {
            open_file(path);
        }
        _ => {
            open_file(path);
        }
    }
} 

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![run_app,list_apps,search_file,open_file,start])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
