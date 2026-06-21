#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::{Manager, State};
use tauri::menu::{Menu, MenuItem};

#[derive(Serialize, Deserialize, Clone, Default)]
struct Settings {
    server_url: Option<String>,
}

const SETTINGS_FILE: &str = "settings.json";

fn load_settings(app: &tauri::AppHandle) -> Settings {
    let dir = match app.path().app_config_dir() { Ok(d) => d, Err(_) => return Settings::default() };
    let path = dir.join(SETTINGS_FILE);
    match std::fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

fn save_settings(app: &tauri::AppHandle, s: &Settings) {
    let dir = match app.path().app_config_dir() { Ok(d) => d, Err(_) => return };
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(json) = serde_json::to_string(s) {
        let _ = std::fs::write(dir.join(SETTINGS_FILE), json);
    }
}

async fn ping(url: &str) -> bool {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    let probe = format!("{}/api/companion/ping", url.trim_end_matches('/'));
    match client.get(&probe).send().await {
        Ok(r) => r.status().is_success() || r.status().as_u16() == 401,
        Err(_) => false,
    }
}

struct AppState {
    local_url: String,
}

fn navigate(w: &tauri::WebviewWindow, url: &str) {
    let js = format!("window.location.replace({:?})", url);
    let _ = w.eval(&js);
}

#[tauri::command]
fn get_stored_url(app: tauri::AppHandle) -> Option<String> {
    load_settings(&app).server_url
}

#[tauri::command]
async fn connect(app: tauri::AppHandle, url: String) -> Result<bool, String> {
    let ok = ping(&url).await;
    if ok {
        save_settings(&app, &Settings { server_url: Some(url.clone()) });
    }
    Ok(ok)
}

#[tauri::command]
fn forget(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    save_settings(&app, &Settings::default());
    if let Some(w) = app.get_webview_window("main") {
        navigate(&w, &state.local_url);
    }
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .menu(|handle| {
            let change = MenuItem::with_id(handle, "change-server", "Change server...", true, None::<&str>)?;
            Menu::with_items(handle, &[&change])
        })
        .on_menu_event(|app, event| {
            if event.id() == "change-server" {
                let state: State<AppState> = app.state();
                if let Some(w) = app.get_webview_window("main") {
                    navigate(&w, &state.local_url);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![get_stored_url, connect, forget])
        .setup(|app| {
            if let Some(w) = app.get_webview_window("main") {
                let local_url = match w.url() { Ok(u) => u.to_string(), Err(_) => String::new() };
                let local_url = if local_url.is_empty() {
                    // Fallback per platform if w.url() ever returns empty.
                    #[cfg(target_os = "windows")]
                    { "http://tauri.localhost/index.html".to_string() }
                    #[cfg(not(target_os = "windows"))]
                    { "tauri://localhost/index.html".to_string() }
                } else {
                    local_url.to_string()
                };
                let local_url = if local_url.ends_with('/') {
                    format!("{}index.html", local_url)
                } else if !local_url.ends_with("index.html") {
                    format!("{}/index.html", local_url)
                } else {
                    local_url
                };
                let state = AppState { local_url: local_url };
                app.manage(state);

                let settings = load_settings(app.handle());
                if let Some(u) = settings.server_url {
                    let w2 = w.clone();
                    let app_h = app.handle().clone();
                    tauri::async_runtime::spawn(async move {
                        if ping(&u).await {
                            navigate(&w2, &u);
                        }
                    });
                    let _ = app_h;
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
