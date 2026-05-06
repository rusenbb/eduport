mod reveal;
mod sidecar;

use std::sync::Mutex;
use std::time::Duration;
use tauri::Manager;

struct SidecarState(Mutex<Option<sidecar::SidecarHandle>>);

#[tauri::command]
fn get_sidecar_url(state: tauri::State<SidecarState>) -> Option<String> {
    state
        .0
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref().map(|h| format!("http://127.0.0.1:{}", h.port)))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .manage(SidecarState(Mutex::new(None)))
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Spawn the sidecar. If it fails (e.g. eduport-sidecar not on PATH),
            // log + let the frontend's status banner take over.
            match sidecar::spawn_sidecar() {
                Ok(handle) => {
                    let port = handle.port;
                    if let Err(e) = sidecar::wait_for_health(port, Duration::from_secs(5)) {
                        log::error!("sidecar health check failed: {}", e);
                    }
                    let url = format!("http://127.0.0.1:{}", port);
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.eval(&format!(
                            "window.__EDUPORT_API_URL__ = '{}';",
                            url
                        ));
                    }
                    if let Ok(mut guard) = app.state::<SidecarState>().0.lock() {
                        *guard = Some(handle);
                    }
                }
                Err(e) => {
                    log::error!("failed to spawn sidecar: {}", e);
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_sidecar_url,
            reveal::reveal_in_file_manager,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                if let Ok(mut guard) = window.app_handle().state::<SidecarState>().0.lock() {
                    if let Some(mut handle) = guard.take() {
                        sidecar::kill_sidecar(&mut handle);
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
