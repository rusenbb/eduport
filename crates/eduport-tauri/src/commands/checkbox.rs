//! Checkbox-toggle command.
//!
//! Tasks-style markdown bodies use `- [ ]` / `- [x]` lines for
//! task items. This command flips the checkbox state on a single
//! line by file_id + line number, rewrites the file, and updates
//! the index.
//!
//! The Python sidecar treated this as part of the FTS5 layer
//! (writing through a `checkboxes` SQL table). Phase 7 dropped
//! that table for being a separate concern; we route through the
//! file directly here, which is simpler and keeps the body
//! truth-source on disk.

use std::path::Path;

use eduport_core::index::writer::upsert_entity as index_upsert;
use serde::{Deserialize, Serialize};
use tauri::State;

use super::{CommandError, require_state};
use crate::core_state::EduportStateHandle;

#[derive(Debug, Deserialize)]
pub struct CheckboxToggleBody {
    pub file_id: String,
    pub line: usize,
    pub checked: bool,
}

#[derive(Debug, Serialize)]
pub struct ToggleResult {
    pub ok: bool,
}

/// Toggle a single checkbox line. The frontend sends the 1-based
/// line number it observed; we look up the file via the index
/// (which has a `path` column for every entity), rewrite the body
/// in place, and refresh the index synchronously.
#[tauri::command]
pub fn core_checkbox_toggle(
    state: State<'_, EduportStateHandle>,
    body: CheckboxToggleBody,
) -> Result<ToggleResult, CommandError> {
    let st = require_state(&state)?;
    let (path_str, kind_str): (String, String) = {
        let index = st
            .index
            .lock()
            .map_err(|_| CommandError::internal("index mutex poisoned"))?;
        index
            .conn()
            .query_row(
                "SELECT path, type FROM entities WHERE file_id = ?1",
                [&body.file_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|_| CommandError::not_found(format!("no entity {:?}", body.file_id)))?
    };
    let kind: eduport_core::EntityType = kind_str
        .parse()
        .map_err(|e: String| CommandError::internal(format!("unknown entity type: {e}")))?;
    let path = std::path::PathBuf::from(path_str);

    let raw = std::fs::read_to_string(&path)?;
    let (frontmatter_block, body_text) = split_with_frontmatter(&raw)
        .ok_or_else(|| CommandError::invalid("file lacks `---` frontmatter delimiters"))?;

    let new_body = toggle_body_line(body_text, body.line, body.checked)?;

    let new_raw = format!("---\n{frontmatter_block}\n---\n{new_body}");

    if let Ok(guard) = st.watcher.lock()
        && let Some(watcher) = guard.as_ref()
    {
        watcher.note_self_write(&path);
    }
    // Atomic write via tempfile + rename. We don't pull in
    // vaultdb-core's `atomic_write` here because eduport-tauri
    // doesn't carry a direct vaultdb-core dep — the same primitive
    // is reachable but only through eduport-core's internal modules,
    // which aren't part of its public surface.
    write_atomically(&path, &new_raw)?;

    // Refresh the index so the FTS5 body picks up the new state.
    let entity = st
        .entity_store
        .find_by_name(kind, &body.file_id)?
        .ok_or_else(|| CommandError::not_found(body.file_id.clone()))?;
    let mtime_ns = path
        .metadata()
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0);
    let schema = st.schema_store.current().ok();
    let index = st
        .index
        .lock()
        .map_err(|_| CommandError::internal("index mutex poisoned"))?;
    index_upsert(
        index.conn(),
        &body.file_id,
        &path,
        mtime_ns,
        &entity,
        &new_body,
        schema.as_ref(),
    )?;

    Ok(ToggleResult { ok: true })
}

/// Split a markdown file into `(yaml_block, body)`. Returns `None`
/// if the leading `---\n` delimiter or its closing `\n---\n` is
/// missing.
fn split_with_frontmatter(raw: &str) -> Option<(&str, &str)> {
    let trimmed = raw.strip_prefix("---\n")?;
    let close = trimmed.find("\n---\n")?;
    Some((&trimmed[..close], &trimmed[close + "\n---\n".len()..]))
}

/// Flip the checkbox marker on the given 1-based line number. Errors
/// when the line is out of range or doesn't carry a checkbox.
fn toggle_body_line(body: &str, line_no: usize, checked: bool) -> Result<String, CommandError> {
    if line_no == 0 {
        return Err(CommandError::invalid("line number must be 1-based"));
    }
    let lines: Vec<&str> = body.split_inclusive('\n').collect();
    if line_no > lines.len() {
        return Err(CommandError::invalid(format!(
            "line {} is past end of body ({} lines)",
            line_no,
            lines.len()
        )));
    }
    let target = lines[line_no - 1];
    let replaced = replace_checkbox_marker(target, checked).ok_or_else(|| {
        CommandError::invalid(format!(
            "line {line_no} does not contain a `[ ]` or `[x]` marker"
        ))
    })?;
    let mut out = String::with_capacity(body.len());
    for (i, l) in lines.iter().enumerate() {
        if i + 1 == line_no {
            out.push_str(&replaced);
        } else {
            out.push_str(l);
        }
    }
    Ok(out)
}

/// Replace the first `[ ]` / `[x]` / `[X]` marker on a single line
/// with the requested state. Preserves indentation and any leading
/// `- ` bullet — we only touch the bracketed character.
fn replace_checkbox_marker(line: &str, checked: bool) -> Option<String> {
    let mut chars: Vec<char> = line.chars().collect();
    for i in 0..chars.len().saturating_sub(2) {
        if chars[i] == '[' && chars[i + 2] == ']' {
            let mid = chars[i + 1];
            if mid == ' ' || mid == 'x' || mid == 'X' {
                chars[i + 1] = if checked { 'x' } else { ' ' };
                return Some(chars.iter().collect());
            }
        }
    }
    None
}

/// Tempfile + same-directory-rename atomic write. Same shape as
/// `vaultdb_core::writer::atomic_write` (which we can't reach
/// without adding vaultdb-core to this crate's deps), reduced to
/// the single use this command needs.
fn write_atomically(path: &Path, content: &str) -> Result<(), CommandError> {
    let parent = path
        .parent()
        .ok_or_else(|| CommandError::internal(format!("path has no parent: {path:?}")))?;
    std::fs::create_dir_all(parent)?;
    let mut tmp = parent.join(
        path.file_name()
            .map(std::ffi::OsString::from)
            .unwrap_or_default(),
    );
    tmp.set_extension("md.tmp");
    std::fs::write(&tmp, content)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_unchecked_to_checked() {
        let out = replace_checkbox_marker("- [ ] do laundry", true).unwrap();
        assert_eq!(out, "- [x] do laundry");
    }

    #[test]
    fn toggle_checked_to_unchecked() {
        let out = replace_checkbox_marker("  - [x] write code", false).unwrap();
        assert_eq!(out, "  - [ ] write code");
    }

    #[test]
    fn rejects_lines_without_checkbox() {
        assert!(replace_checkbox_marker("a normal line", true).is_none());
    }

    #[test]
    fn toggle_body_line_replaces_correct_line() {
        let body = "- [ ] one\n- [ ] two\n- [ ] three\n";
        let new_body = toggle_body_line(body, 2, true).unwrap();
        assert_eq!(new_body, "- [ ] one\n- [x] two\n- [ ] three\n");
    }

    #[test]
    fn out_of_range_line_errors() {
        let body = "- [ ] only one\n";
        let err = toggle_body_line(body, 5, true).unwrap_err();
        assert_eq!(err.code, "invalid");
    }
}
