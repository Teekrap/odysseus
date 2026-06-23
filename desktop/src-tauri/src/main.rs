#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use tauri::menu::{Menu, MenuItem};

#[derive(Serialize, Deserialize, Clone, Default)]
struct Settings {
    server_url: Option<String>,
}

const SETTINGS_FILE: &str = "settings.json";
const SETTINGS_WIN: &str = "settings";
const MAIN_WIN: &str = "main";

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

async fn probe(url: &str) -> Result<u16, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .redirect(reqwest::redirect::Policy::limited(3))
        .build()
        .map_err(|e| e.to_string())?;
    let probe_url = url.trim_end_matches('/').to_string();
    match client.get(&probe_url).send().await {
        Ok(r) => Ok(r.status().as_u16()),
        Err(e) => {
            if e.is_timeout() {
                Err("Server did not respond within 8 seconds.".into())
            } else if e.is_connect() {
                Err(format!("Cannot reach server: {}", e))
            } else {
                Err(format!("Probe error: {}", e))
            }
        }
    }
}

fn open_settings(app: &tauri::AppHandle) {
    if let Some(existing) = app.get_webview_window(SETTINGS_WIN) {
        let _ = existing.show();
        let _ = existing.set_focus();
        return;
    }
    let _ = WebviewWindowBuilder::new(app, SETTINGS_WIN, WebviewUrl::App("index.html".into()))
        .title("Odysseus")
        .inner_size(480.0, 360.0)
        .min_inner_size(360.0, 280.0)
        .resizable(true)
        .center()
        .visible(true)
        .build();
}

fn open_server(app: &tauri::AppHandle, url: &str) {
    if let Some(old) = app.get_webview_window(MAIN_WIN) {
        let _ = old.close();
    }
    let parsed = match tauri::Url::parse(url) {
        Ok(u) => u,
        Err(_) => return,
    };
    if let Ok(w) = WebviewWindowBuilder::new(app, MAIN_WIN, WebviewUrl::External(parsed))
        .title("Odysseus")
        .inner_size(1200.0, 800.0)
        .min_inner_size(480.0, 320.0)
        .resizable(true)
        .center()
        .visible(true)
        .build()
    {
        let _ = w.show();
        let _ = w.set_focus();
        if let Some(s) = app.get_webview_window(SETTINGS_WIN) {
            let _ = s.close();
        }
    }
}

#[tauri::command]
fn get_stored_url(app: tauri::AppHandle) -> Option<String> {
    load_settings(&app).server_url
}

#[tauri::command]
async fn connect(app: tauri::AppHandle, url: String) -> Result<bool, String> {
    probe(&url).await?;
    save_settings(&app, &Settings { server_url: Some(url.clone()) });
    open_server(&app, &url);
    Ok(true)
}

#[tauri::command]
fn forget(app: tauri::AppHandle) -> Result<(), String> {
    save_settings(&app, &Settings::default());
    if let Some(w) = app.get_webview_window(MAIN_WIN) {
        let _ = w.close();
    }
    open_settings(&app);
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
                if let Some(w) = app.get_webview_window(MAIN_WIN) {
                    let _ = w.close();
                }
                open_settings(app);
            }
        })
        .invoke_handler(tauri::generate_handler![get_stored_url, connect, forget])
        .setup(|app| {
            let settings = load_settings(app.handle());
            match settings.server_url {
                Some(url) => {
                    let app_h = app.handle().clone();
                    tauri::async_runtime::spawn(async move {
                        match probe(&url).await {
                            Ok(_) => open_server(&app_h, &url),
                            Err(_) => open_settings(&app_h),
                        }
                    });
                }
                None => {
                    open_settings(app.handle());
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
