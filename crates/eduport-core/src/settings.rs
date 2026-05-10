//! User settings persistence. The settings file lives outside the
//! vault — typically at the OS-conventional config location —
//! because it tells eduport *which* vault to open. Eduport-tauri owns
//! the path resolution; this module just handles reading/writing the
//! TOML file.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::EduportError;

/// Eduport's user-visible settings. Round-trips through TOML.
///
/// `data_folder` is the Obsidian vault root (becomes the
/// `vaultdb-core::Vault::root` once eduport-core wires up the data
/// layer). Sub-folders for `attachments_folder` and `notes_folder` are
/// resolved relative to it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub data_folder: PathBuf,
    #[serde(default = "default_attachments_folder")]
    pub attachments_folder: String,
    #[serde(default = "default_notes_folder")]
    pub notes_folder: String,
    #[serde(default)]
    pub theme: Theme,
    pub user_email: String,
    #[serde(default = "default_zoom_factor")]
    pub zoom_factor: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub obsidian_vault: Option<String>,
    #[serde(default = "default_confirm_deletes")]
    pub confirm_deletes: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

fn default_attachments_folder() -> String {
    "./attachments".into()
}
fn default_notes_folder() -> String {
    "./notes".into()
}
fn default_zoom_factor() -> f64 {
    1.0
}
fn default_confirm_deletes() -> bool {
    true
}

impl Settings {
    /// Validate the in-memory shape. Called by [`load_settings`] after
    /// deserialising and before returning. Catches things `serde` can't
    /// (e.g. zoom_factor out of range, blank obsidian_vault → None
    /// normalisation).
    pub fn normalize(&mut self) -> Result<(), EduportError> {
        if !(0.75..=1.5).contains(&self.zoom_factor) {
            return Err(EduportError::Schema(format!(
                "zoom_factor {} out of range [0.75, 1.5]",
                self.zoom_factor
            )));
        }
        if let Some(s) = &self.obsidian_vault
            && s.trim().is_empty()
        {
            self.obsidian_vault = None;
        }
        Ok(())
    }

    pub fn resolved_attachments_folder(&self) -> PathBuf {
        self.data_folder.join(&self.attachments_folder)
    }

    pub fn resolved_notes_folder(&self) -> PathBuf {
        self.data_folder.join(&self.notes_folder)
    }
}

/// Load settings from a TOML file. Returns `Ok(None)` if the file
/// doesn't exist (caller decides whether to seed or prompt the user).
/// Errors on malformed TOML or invariant violations.
pub fn load_settings(path: &Path) -> Result<Option<Settings>, EduportError> {
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(path).map_err(EduportError::Io)?;
    let mut settings: Settings =
        toml::from_str(&text).map_err(|e| EduportError::Schema(format!("settings.toml: {}", e)))?;
    settings.normalize()?;
    Ok(Some(settings))
}

/// Save settings to a TOML file atomically. Writes to a tempfile in
/// the same directory and renames over the target — readers never see
/// a partial settings file. Uses [`vaultdb_core::writer::atomic_write`]
/// for the rename, the same primitive that backs every vaultdb-core
/// mutation.
pub fn save_settings(settings: &Settings, path: &Path) -> Result<(), EduportError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(EduportError::Io)?;
    }
    let text = toml::to_string_pretty(settings)
        .map_err(|e| EduportError::Schema(format!("settings serialize: {}", e)))?;
    vaultdb_core::writer::atomic_write(path, &text).map_err(EduportError::Io)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_settings(data_folder: PathBuf) -> Settings {
        Settings {
            data_folder,
            attachments_folder: "./attachments".into(),
            notes_folder: "./notes".into(),
            theme: Theme::Light,
            user_email: "user@example.com".into(),
            zoom_factor: 1.2,
            obsidian_vault: None,
            confirm_deletes: true,
        }
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = TempDir::new().unwrap();
        let settings_path = dir.path().join("settings.toml");
        let original = sample_settings(dir.path().join("vault"));

        save_settings(&original, &settings_path).unwrap();
        let loaded = load_settings(&settings_path).unwrap().unwrap();
        assert_eq!(loaded, original);
    }

    #[test]
    fn load_returns_none_when_file_missing() {
        let dir = TempDir::new().unwrap();
        let result = load_settings(&dir.path().join("absent.toml")).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn defaults_apply_when_fields_omitted() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("partial.toml");
        // Only the required fields. The rest get default values via serde.
        std::fs::write(
            &path,
            r#"
data_folder = "/some/vault"
user_email = "a@b.c"
            "#,
        )
        .unwrap();
        let s = load_settings(&path).unwrap().unwrap();
        assert_eq!(s.attachments_folder, "./attachments");
        assert_eq!(s.notes_folder, "./notes");
        assert!(matches!(s.theme, Theme::System));
        assert_eq!(s.zoom_factor, 1.0);
        assert!(s.confirm_deletes);
        assert_eq!(s.obsidian_vault, None);
    }

    #[test]
    fn rejects_unknown_fields_on_load() {
        // deny_unknown_fields catches typos at load time so bad
        // settings.toml doesn't silently turn into surprising defaults.
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("bad.toml");
        std::fs::write(
            &path,
            r#"
data_folder = "/v"
user_email = "a@b"
typo_field = "oops"
            "#,
        )
        .unwrap();
        assert!(load_settings(&path).is_err());
    }

    #[test]
    fn rejects_zoom_out_of_range() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("bad.toml");
        std::fs::write(
            &path,
            r#"
data_folder = "/v"
user_email = "a@b"
zoom_factor = 2.5
            "#,
        )
        .unwrap();
        assert!(load_settings(&path).is_err());
    }

    #[test]
    fn blank_obsidian_vault_normalised_to_none() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("blank.toml");
        std::fs::write(
            &path,
            r#"
data_folder = "/v"
user_email = "a@b"
obsidian_vault = "   "
            "#,
        )
        .unwrap();
        let s = load_settings(&path).unwrap().unwrap();
        assert_eq!(s.obsidian_vault, None);
    }

    #[test]
    fn save_creates_parent_directory() {
        let dir = TempDir::new().unwrap();
        let nested = dir.path().join("a/b/c/settings.toml");
        let original = sample_settings(dir.path().join("vault"));
        save_settings(&original, &nested).unwrap();
        assert!(nested.is_file());
    }

    #[test]
    fn resolved_paths_are_relative_to_data_folder() {
        let dir = TempDir::new().unwrap();
        let s = sample_settings(dir.path().join("vault"));
        assert_eq!(
            s.resolved_attachments_folder(),
            dir.path().join("vault/./attachments")
        );
        assert_eq!(s.resolved_notes_folder(), dir.path().join("vault/./notes"));
    }
}
