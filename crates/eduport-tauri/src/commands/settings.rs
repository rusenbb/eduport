//! Settings get/put commands.
//!
//! Settings live in the Tauri app config dir at `settings.toml`.
//! `core_settings_get` returns the current persisted value;
//! `core_settings_put` writes it atomically and rebuilds the
//! `EduportState` if the data folder changed.

use std::path::PathBuf;

use eduport_core::{Settings, load_settings, save_settings};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use super::CommandError;
use crate::core_state::{self, EduportStateHandle};

/// Frontend-shaped settings DTO. The frontend uses string types
/// for everything (it doesn't know `PathBuf`), so we accept/return
/// strings here and convert at the boundary.
#[derive(Debug, Serialize, Deserialize)]
pub struct SettingsDto {
    pub data_folder: String,
    pub attachments_folder: String,
    pub notes_folder: String,
    pub theme: String,
    pub user_email: String,
    pub zoom_factor: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub obsidian_vault: Option<String>,
    pub confirm_deletes: bool,
}

impl From<&Settings> for SettingsDto {
    fn from(s: &Settings) -> Self {
        Self {
            data_folder: s.data_folder.to_string_lossy().into_owned(),
            attachments_folder: s.attachments_folder.clone(),
            notes_folder: s.notes_folder.clone(),
            theme: theme_to_str(&s.theme).into(),
            user_email: s.user_email.clone(),
            zoom_factor: s.zoom_factor,
            obsidian_vault: s.obsidian_vault.clone(),
            confirm_deletes: s.confirm_deletes,
        }
    }
}

impl SettingsDto {
    fn into_settings(self) -> Result<Settings, CommandError> {
        let theme = match self.theme.as_str() {
            "light" => eduport_core::Theme::Light,
            "dark" => eduport_core::Theme::Dark,
            "system" => eduport_core::Theme::System,
            other => return Err(CommandError::invalid(format!("unknown theme: {other}"))),
        };
        let mut settings = Settings {
            data_folder: PathBuf::from(self.data_folder),
            attachments_folder: self.attachments_folder,
            notes_folder: self.notes_folder,
            theme,
            user_email: self.user_email,
            zoom_factor: self.zoom_factor,
            obsidian_vault: self.obsidian_vault,
            confirm_deletes: self.confirm_deletes,
        };
        settings
            .normalize()
            .map_err(|e| CommandError::invalid(e.to_string()))?;
        Ok(settings)
    }
}

fn theme_to_str(t: &eduport_core::Theme) -> &'static str {
    match t {
        eduport_core::Theme::Light => "light",
        eduport_core::Theme::Dark => "dark",
        eduport_core::Theme::System => "system",
    }
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, CommandError> {
    Ok(app
        .path()
        .app_config_dir()
        .map_err(|e| CommandError::internal(format!("config dir: {e}")))?
        .join("settings.toml"))
}

#[tauri::command]
pub fn core_settings_get(app: AppHandle) -> Result<SettingsDto, CommandError> {
    let path = settings_path(&app)?;
    let settings = load_settings(&path)
        .map_err(CommandError::from)?
        .ok_or_else(|| {
            CommandError::not_found(format!(
                "no settings file at {}; run first-run setup",
                path.display()
            ))
        })?;
    Ok((&settings).into())
}

/// Persist new settings and reboot the eduport-core state.
///
/// Reboot is unconditional rather than conditional on
/// `data_folder` changing because (a) the user_email is captured
/// inside `EduportState` for EML import discrimination, (b)
/// folder overrides could become reachable later, and (c) the
/// reboot is cheap (drop watcher → open new index → reconcile;
/// sub-second on a vault under our scale targets).
#[tauri::command]
pub fn core_settings_put(
    app: AppHandle,
    state: State<'_, EduportStateHandle>,
    settings: SettingsDto,
) -> Result<SettingsDto, CommandError> {
    let path = settings_path(&app)?;
    let parsed = settings.into_settings()?;
    save_settings(&parsed, &path).map_err(CommandError::from)?;

    // Drop the existing state (stops the watcher) and rebuild from
    // the new settings.
    core_state::shutdown(&state);
    let fresh = core_state::build_state(&app, &parsed)
        .map_err(|e| CommandError::internal(e.to_string()))?;
    *state
        .lock()
        .map_err(|_| CommandError::internal("state handle poisoned"))? = Some(fresh);

    Ok((&parsed).into())
}
