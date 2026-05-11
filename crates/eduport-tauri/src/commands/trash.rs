//! Trash commands.
//!
//! `EntityStore::delete(.., false)` moves files to vaultdb's
//! `<vault>/.trash/` directory. These commands list, restore, and
//! purge that directory.
//!
//! Restore strategy: open the file, parse its frontmatter to read
//! the `eduport-type/<value>` discriminator, look up the right
//! folder in the FolderMap, and move the file back. This is more
//! robust than the Python sidecar's "sidecar metadata file"
//! approach — the file's content is its own metadata.

use std::path::{Path, PathBuf};

use eduport_core::entity::Entity;
use serde::{Deserialize, Serialize};
use tauri::State;

use super::{CommandError, require_state};
use crate::core_state::EduportStateHandle;

const TRASH_DIR_NAME: &str = ".trash";

#[derive(Debug, Serialize)]
pub struct TrashItem {
    pub name: String,
    pub path: String,
    pub original_path: Option<String>,
    pub size: u64,
    pub modified: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RestoreBody {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct RestoreResult {
    pub path: String,
    pub file_id: String,
}

/// List every `.md` file in the trash directory. `original_path`
/// is inferred from the file's `eduport-type/<value>` tag — when
/// that's missing or unparseable, we fall back to `None` rather
/// than failing the whole listing.
#[tauri::command]
pub fn core_trash_list(
    state: State<'_, EduportStateHandle>,
) -> Result<Vec<TrashItem>, CommandError> {
    let st = require_state(&state)?;
    let dir = st.data_folder.join(TRASH_DIR_NAME);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut items = Vec::new();
    let entries = std::fs::read_dir(&dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        let metadata = entry.metadata()?;
        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| format_iso8601_seconds(d.as_secs()));
        let original_path = infer_original_path(&st, &path);
        items.push(TrashItem {
            name: entry.file_name().to_string_lossy().into_owned(),
            path: path.to_string_lossy().into_owned(),
            original_path: original_path.map(|p| p.to_string_lossy().into_owned()),
            size: metadata.len(),
            modified,
        });
    }
    items.sort_by_key(|a| a.name.to_lowercase());
    Ok(items)
}

/// Restore a single file from the trash by inferring its original
/// folder from the file's entity-type discriminator.
#[tauri::command]
pub fn core_trash_restore(
    state: State<'_, EduportStateHandle>,
    body: RestoreBody,
) -> Result<RestoreResult, CommandError> {
    let st = require_state(&state)?;
    let trash_path = trash_path_for(&st.data_folder, &body.name)?;
    if !trash_path.exists() {
        return Err(CommandError::not_found(format!(
            "no item {:?} in trash",
            body.name
        )));
    }
    let original = infer_original_path(&st, &trash_path).ok_or_else(|| {
        CommandError::invalid(
            "could not infer original location: file lacks an `eduport-type/<value>` tag",
        )
    })?;
    if let Some(parent) = original.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::rename(&trash_path, &original)?;
    let file_id = original
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string();
    Ok(RestoreResult {
        path: original.to_string_lossy().into_owned(),
        file_id,
    })
}

/// Permanently delete a single trashed file.
#[tauri::command]
pub fn core_trash_delete(
    state: State<'_, EduportStateHandle>,
    name: String,
) -> Result<(), CommandError> {
    let st = require_state(&state)?;
    let trash_path = trash_path_for(&st.data_folder, &name)?;
    if !trash_path.exists() {
        return Err(CommandError::not_found(format!(
            "no item {:?} in trash",
            name
        )));
    }
    std::fs::remove_file(&trash_path)?;
    Ok(())
}

/// Permanently empty the trash directory.
#[tauri::command]
pub fn core_trash_empty(state: State<'_, EduportStateHandle>) -> Result<(), CommandError> {
    let st = require_state(&state)?;
    let dir = st.data_folder.join(TRASH_DIR_NAME);
    if !dir.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let _ = std::fs::remove_file(entry.path());
    }
    Ok(())
}

// ── helpers ─────────────────────────────────────────────────────────

/// Resolve `name` to a path inside the trash directory, refusing
/// any input that escapes the directory (path traversal safety —
/// matches the Python sidecar's `_safe_trash_path`).
fn trash_path_for(data_folder: &Path, name: &str) -> Result<PathBuf, CommandError> {
    let dir = data_folder.join(TRASH_DIR_NAME);
    let candidate = dir.join(name);
    let canonical_dir = std::fs::canonicalize(&dir).unwrap_or(dir.clone());
    let canonical_candidate =
        std::fs::canonicalize(&candidate).unwrap_or_else(|_| candidate.clone());
    if !canonical_candidate.starts_with(&canonical_dir) {
        return Err(CommandError::invalid(format!(
            "trash item name escapes trash directory: {name:?}"
        )));
    }
    Ok(candidate)
}

/// Compute the path a trashed file would land at on restore.
/// All entities live flat at the vault root, so the destination
/// is simply `<data_folder>/<stem>.md`. We still parse the
/// frontmatter to verify the file *is* an eduport entity — a
/// stray non-entity .md that wandered into `.trash/` shouldn't
/// restore.
fn infer_original_path(state: &crate::core_state::EduportState, trashed: &Path) -> Option<PathBuf> {
    // Parse purely as a validity check — the entity type tag must be
    // present for this to be a real eduport entity. Routed through
    // vaultdb's canonical loader so we share the frontmatter parser.
    let record = vaultdb_core::frontmatter::load_record(trashed).ok()?;
    let _entity = Entity::from_record(&record, &state.data_folder).ok()?;
    let stem = trashed.file_stem()?.to_str()?;
    Some(state.data_folder.join(format!("{stem}.md")))
}

/// Format a Unix timestamp as ISO-8601 in UTC. We avoid pulling in
/// chrono just for this — the tradeoff is no fractional seconds.
fn format_iso8601_seconds(secs: u64) -> String {
    // Days since 1970-01-01.
    let days = (secs / 86_400) as i64;
    let mut secs_of_day = (secs % 86_400) as i64;
    let hour = secs_of_day / 3_600;
    secs_of_day %= 3_600;
    let minute = secs_of_day / 60;
    let second = secs_of_day % 60;

    // Civil-from-days conversion (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = y + (m <= 2) as i64;
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y, m, d, hour, minute, second
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso8601_for_known_epoch_values() {
        assert_eq!(format_iso8601_seconds(0), "1970-01-01T00:00:00Z");
        // Sanity: the Unix epoch + 1 day is 1970-01-02 00:00:00 UTC.
        assert_eq!(format_iso8601_seconds(86_400), "1970-01-02T00:00:00Z");
        // A 2020 date — calculated with the same algorithm.
        // 2020-06-15 12:34:56 UTC = 1592224496
        assert_eq!(
            format_iso8601_seconds(1_592_224_496),
            "2020-06-15T12:34:56Z"
        );
    }
}
