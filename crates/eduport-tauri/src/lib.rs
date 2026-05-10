mod commands;
mod core_state;
mod reveal;
mod sidecar;

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;
use tauri::Manager;

use crate::core_state::EduportStateHandle;

struct SidecarState(Mutex<Option<sidecar::SidecarHandle>>);

#[derive(Serialize)]
struct BootstrapStatus {
    settings_exists: bool,
    settings_path: String,
    sidecar_url: Option<String>,
}

#[derive(Deserialize, Serialize)]
struct BootstrapSettings {
    data_folder: String,
    #[serde(default = "default_attachments_folder")]
    attachments_folder: String,
    #[serde(default = "default_notes_folder")]
    notes_folder: String,
    #[serde(default = "default_theme")]
    theme: String,
    user_email: String,
    #[serde(default = "default_zoom_factor")]
    zoom_factor: f64,
    #[serde(default)]
    obsidian_vault: Option<String>,
    #[serde(default = "default_confirm_deletes")]
    confirm_deletes: bool,
}

fn default_attachments_folder() -> String {
    "./attachments".to_string()
}

fn default_notes_folder() -> String {
    "./notes".to_string()
}

fn default_theme() -> String {
    "system".to_string()
}

fn default_zoom_factor() -> f64 {
    1.0
}

fn default_confirm_deletes() -> bool {
    true
}

#[tauri::command]
fn get_sidecar_url(state: tauri::State<SidecarState>) -> Option<String> {
    sidecar_url_from_state(&state)
}

fn sidecar_url_from_state(state: &tauri::State<SidecarState>) -> Option<String> {
    state.0.lock().ok().and_then(|guard| {
        guard
            .as_ref()
            .map(|h| format!("http://127.0.0.1:{}", h.port))
    })
}

fn settings_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|dir| dir.join("settings.toml"))
        .map_err(|e| format!("failed to resolve app config directory: {e}"))
}

fn folder_from_setting(data_folder: &Path, configured: &str) -> PathBuf {
    let configured = Path::new(configured);
    if configured.is_absolute() {
        configured.to_path_buf()
    } else {
        data_folder.join(configured)
    }
}

fn write_settings_file(app: &tauri::AppHandle, settings: &BootstrapSettings) -> Result<(), String> {
    let settings_path = settings_path(app)?;
    if let Some(parent) = settings_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create settings directory: {e}"))?;
    }

    let data_folder = PathBuf::from(&settings.data_folder);
    fs::create_dir_all(&data_folder).map_err(|e| format!("failed to create data folder: {e}"))?;
    fs::create_dir_all(folder_from_setting(
        &data_folder,
        &settings.attachments_folder,
    ))
    .map_err(|e| format!("failed to create attachments folder: {e}"))?;
    fs::create_dir_all(folder_from_setting(&data_folder, &settings.notes_folder))
        .map_err(|e| format!("failed to create notes folder: {e}"))?;

    let payload =
        toml::to_string_pretty(settings).map_err(|e| format!("failed to encode settings: {e}"))?;
    fs::write(&settings_path, payload).map_err(|e| format!("failed to write settings: {e}"))?;
    Ok(())
}

fn read_settings_file(app: &tauri::AppHandle) -> Result<Option<BootstrapSettings>, String> {
    let settings_path = settings_path(app)?;
    if !settings_path.exists() {
        return Ok(None);
    }
    let raw =
        fs::read_to_string(&settings_path).map_err(|e| format!("failed to read settings: {e}"))?;
    toml::from_str(&raw)
        .map(Some)
        .map_err(|e| format!("failed to parse settings: {e}"))
}

fn sanitize_zoom(zoom_factor: f64) -> f64 {
    if zoom_factor.is_finite() {
        zoom_factor.clamp(0.75, 1.5)
    } else {
        1.0
    }
}

fn apply_zoom(window: &tauri::WebviewWindow, zoom_factor: f64) -> Result<(), String> {
    window
        .set_zoom(sanitize_zoom(zoom_factor))
        .map_err(|e| format!("failed to apply zoom: {e}"))
}

fn ensure_sidecar(
    app: &tauri::AppHandle,
    state: &tauri::State<SidecarState>,
) -> Result<String, String> {
    if let Some(url) = sidecar_url_from_state(state) {
        return Ok(url);
    }

    let settings_path = settings_path(app)?;
    if !settings_path.exists() {
        return Err(format!(
            "settings file does not exist yet: {}",
            settings_path.display()
        ));
    }

    let handle = sidecar::spawn_sidecar(app, &settings_path)?;
    let port = handle.port;
    if let Err(e) = sidecar::wait_for_health(port, Duration::from_secs(8)) {
        sidecar::kill_sidecar(handle);
        return Err(e);
    }

    if let Ok(mut guard) = state.0.lock() {
        *guard = Some(handle);
    }

    Ok(format!("http://127.0.0.1:{port}"))
}

#[tauri::command]
fn get_bootstrap_status(
    app: tauri::AppHandle,
    state: tauri::State<SidecarState>,
) -> Result<BootstrapStatus, String> {
    let settings_path = settings_path(&app)?;
    Ok(BootstrapStatus {
        settings_exists: settings_path.exists(),
        settings_path: settings_path.to_string_lossy().into_owned(),
        sidecar_url: sidecar_url_from_state(&state),
    })
}

#[tauri::command]
fn ensure_sidecar_started(
    app: tauri::AppHandle,
    state: tauri::State<SidecarState>,
) -> Result<String, String> {
    ensure_sidecar(&app, &state)
}

#[tauri::command]
fn bootstrap_settings(
    app: tauri::AppHandle,
    state: tauri::State<SidecarState>,
    window: tauri::WebviewWindow,
    settings: BootstrapSettings,
) -> Result<String, String> {
    write_settings_file(&app, &settings)?;
    apply_zoom(&window, settings.zoom_factor)?;
    ensure_sidecar(&app, &state)
}

#[tauri::command]
fn set_app_zoom(window: tauri::WebviewWindow, zoom_factor: f64) -> Result<(), String> {
    apply_zoom(&window, zoom_factor)
}

/// Try to boot eduport-core from the persisted settings.
///
/// Failure is logged at error level rather than aborting startup;
/// the user can still complete first-run setup or fix a corrupted
/// settings file from the GUI. The Phase 9 cutover keeps the
/// sidecar running in parallel until Phase 11 deletes it, so the
/// app remains functional even if the new state failed to boot.
fn try_boot_eduport_core(app: &tauri::AppHandle) {
    let state_handle = app.state::<EduportStateHandle>();
    let path = match settings_path(app) {
        Ok(p) => p,
        Err(e) => {
            log::error!("settings_path: {e}");
            return;
        }
    };
    if !path.exists() {
        log::info!("eduport-core boot deferred: no settings file at {}", path.display());
        return;
    }
    let settings = match eduport_core::load_settings(&path) {
        Ok(Some(s)) => s,
        Ok(None) => {
            log::info!("settings file empty; waiting for first-run setup");
            return;
        }
        Err(e) => {
            log::error!("eduport-core settings load failed: {e}");
            return;
        }
    };
    match core_state::build_state(app, &settings) {
        Ok(state) => {
            if let Ok(mut guard) = state_handle.lock() {
                *guard = Some(state);
                log::info!("eduport-core booted from {}", path.display());
            }
        }
        Err(e) => log::error!("eduport-core build_state failed: {e}"),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .manage(SidecarState(Mutex::new(None)))
        // The eduport-core state is `Mutex<Option<Arc<...>>>` so it
        // can be lazily populated after first-run setup completes.
        .manage::<EduportStateHandle>(Mutex::new(None))
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            match settings_path(app.handle()) {
                Ok(path) if path.exists() => {
                    if let Ok(Some(settings)) = read_settings_file(app.handle()) {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = apply_zoom(&window, settings.zoom_factor);
                        }
                    }
                    let state = app.state::<SidecarState>();
                    match ensure_sidecar(app.handle(), &state) {
                        Ok(url) => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ =
                                    window.eval(format!("window.__EDUPORT_API_URL__ = '{}';", url));
                            }
                        }
                        Err(e) => log::error!("failed to start sidecar: {}", e),
                    }
                }
                Ok(path) => {
                    log::info!(
                        "settings file not found at {}; waiting for first-run setup",
                        path.display()
                    );
                }
                Err(e) => log::error!("{}", e),
            }

            // Boot eduport-core in parallel with the legacy sidecar.
            // Phases 10 & 11 progressively retire the sidecar; this
            // dual-boot is the migration window.
            try_boot_eduport_core(app.handle());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Legacy sidecar bootstrap (Phase 11 deletes these along
            // with the sidecar.rs / sidecar process).
            bootstrap_settings,
            ensure_sidecar_started,
            get_bootstrap_status,
            get_sidecar_url,
            set_app_zoom,
            reveal::copy_file,
            reveal::open_path,
            reveal::read_file_bytes,
            reveal::reveal_in_file_manager,
            // eduport-core entity CRUD
            commands::entity::core_entity_list,
            commands::entity::core_entity_get,
            commands::entity::core_entity_resolve,
            commands::entity::core_entity_create,
            commands::entity::core_entity_update,
            commands::entity::core_entity_delete,
            // eduport-core schema editor
            commands::schema::core_schema_get,
            commands::schema::core_schema_get_type,
            commands::schema::core_schema_add_property,
            commands::schema::core_schema_patch_property,
            commands::schema::core_schema_delete_property,
            commands::schema::core_schema_reorder_properties,
            commands::schema::core_schema_apply_tier_template,
            commands::schema::core_schema_purge_orphans,
            // eduport-core saved views
            commands::view::core_view_get_all,
            commands::view::core_view_get_for_type,
            commands::view::core_view_create,
            commands::view::core_view_update,
            commands::view::core_view_delete,
            commands::view::core_view_reorder,
            // eduport-core search + property aggregations
            commands::search::core_search,
            commands::properties::core_property_value_counts,
            commands::properties::core_filter_entities_by_properties,
            // eduport-core status + counts + tags + parse errors
            commands::status::core_get_status,
            commands::status::core_list_parse_errors,
            commands::status::core_get_counts,
            commands::status::core_get_tags,
            // eduport-core settings (reboots the state on put)
            commands::settings::core_settings_get,
            commands::settings::core_settings_put,
            // eduport-core EML parsing
            commands::eml::core_parse_eml,
            // eduport-core trash management
            commands::trash::core_trash_list,
            commands::trash::core_trash_restore,
            commands::trash::core_trash_delete,
            commands::trash::core_trash_empty,
            // eduport-core checkbox toggle for tasks
            commands::checkbox::core_checkbox_toggle,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                if let Ok(mut guard) = window.app_handle().state::<SidecarState>().0.lock() {
                    if let Some(handle) = guard.take() {
                        sidecar::kill_sidecar(handle);
                    }
                }
                // Stop the eduport-core watcher so its threads
                // shut down cleanly.
                core_state::shutdown(&window.app_handle().state::<EduportStateHandle>());
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
