mod commands;
mod core_state;
mod reveal;

use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Manager;

use crate::core_state::EduportStateHandle;

#[derive(Serialize)]
struct BootstrapStatus {
    settings_exists: bool,
    settings_path: String,
    /// `true` once the eduport-core state has finished its boot
    /// reconcile and is ready to serve commands. Lets the frontend
    /// distinguish "no settings yet" from "settings exist but the
    /// vault is still loading".
    core_ready: bool,
}

fn settings_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|dir| dir.join("settings.toml"))
        .map_err(|e| format!("failed to resolve app config directory: {e}"))
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

#[tauri::command]
fn core_bootstrap_status(
    app: tauri::AppHandle,
    state: tauri::State<EduportStateHandle>,
) -> Result<BootstrapStatus, String> {
    let path = settings_path(&app)?;
    let core_ready = state.lock().ok().map(|g| g.is_some()).unwrap_or(false);
    Ok(BootstrapStatus {
        settings_exists: path.exists(),
        settings_path: path.to_string_lossy().into_owned(),
        core_ready,
    })
}

#[tauri::command]
fn set_app_zoom(window: tauri::WebviewWindow, zoom_factor: f64) -> Result<(), String> {
    apply_zoom(&window, zoom_factor)
}

/// Try to boot eduport-core from the persisted settings.
///
/// Failure is logged at error level rather than aborting startup;
/// the user can still complete first-run setup or fix a corrupted
/// settings file from the GUI.
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
        log::info!(
            "eduport-core boot deferred: no settings file at {}",
            path.display()
        );
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

    if let Some(window) = app.get_webview_window("main") {
        let _ = apply_zoom(&window, settings.zoom_factor);
    }

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

            try_boot_eduport_core(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Bootstrap + zoom + reveal helpers (kept after the
            // sidecar removal — they're host-shell commands, not
            // sidecar API).
            core_bootstrap_status,
            set_app_zoom,
            reveal::copy_file,
            reveal::open_path,
            reveal::read_file_bytes,
            reveal::reveal_in_file_manager,
            // eduport-core entity CRUD
            commands::entity::core_entity_list,
            commands::entity::core_entity_children,
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
                // Stop the eduport-core watcher so its threads
                // shut down cleanly.
                core_state::shutdown(&window.app_handle().state::<EduportStateHandle>());
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
