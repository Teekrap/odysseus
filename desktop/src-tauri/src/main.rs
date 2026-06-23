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

#[derive(Debug)]
enum ProbeError {
    Network(String),
    Timeout,
    Other(String),
}

async fn probe(url: &str) -> Result<u16, ProbeError> {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .redirect(reqwest::redirect::Policy::limited(3))
        .build()
    {
        Ok(c) => c,
        Err(e) => return Err(ProbeError::Other(e.to_string())),
    };
    let probe_url = url.trim_end_matches('/').to_string();
    match client.get(&probe_url).send().await {
        Ok(r) => Ok(r.status().as_u16()),
        Err(e) => {
            if e.is_timeout() {
                Err(ProbeError::Timeout)
            } else if e.is_connect() {
                Err(ProbeError::Network(e.to_string()))
            } else {
                Err(ProbeError::Other(e.to_string()))
            }
        }
    }
}

fn navigate_to(w: &tauri::WebviewWindow, url: &str) {
    let js = format!("window.location.replace({:?})", url);
    let _ = w.eval(&js);
}

struct AppState {
    local_url: String,
}

#[tauri::command]
fn get_stored_url(app: tauri::AppHandle) -> Option<String> {
    load_settings(&app).server_url
}

#[tauri::command]
async fn connect(app: tauri::AppHandle, url: String) -> Result<bool, String> {
    match probe(&url).await {
        Ok(_status) => {
            save_settings(&app, &Settings { server_url: Some(url.clone()) });
            if let Some(w) = app.get_webview_window("main") {
                navigate_to(&w, &url);
            }
            Ok(true)
        }
        Err(ProbeError::Timeout) => Err("Server did not respond within 8 seconds. Check the URL and that Odysseus is reachable from this machine.".into()),
        Err(ProbeError::Network(e)) => Err(format!("Cannot reach server: {}", e)),
        Err(ProbeError::Other(e)) => Err(format!("Probe error: {}", e)),
    }
}

#[tauri::command]
fn forget(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    save_settings(&app, &Settings::default());
    if let Some(w) = app.get_webview_window("main") {
        navigate_to(&w, &state.local_url);
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
                    navigate_to(&w, &state.local_url);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![get_stored_url, connect, forget])
        .setup(|app| {
            if let Some(w) = app.get_webview_window("main") {
                let local_url = match w.url() { Ok(u) => u.to_string(), Err(_) => String::new() };
                let local_url = if local_url.is_empty() {
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
                    tauri::async_runtime::spawn(async move {
                        if probe(&u).await.is_ok() {
                            navigate_to(&w2, &u);
                        }
                    });
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
