//! FTS5 search commands. Wraps `eduport_core::index::search_fts`.

use eduport_core::index::{SearchHit, search_fts};
use serde::Serialize;
use tauri::State;

use super::{CommandError, require_state};
use crate::core_state::EduportStateHandle;

/// Search hit DTO. Same field set as the Python sidecar's response,
/// with `type` instead of Rust's reserved `entity_type` so the
/// frontend's existing parser doesn't need a rename.
#[derive(Debug, Serialize, specta::Type)]
pub struct SearchHitDto {
    pub file_id: String,
    #[serde(rename = "type")]
    pub entity_type: String,
    pub name: String,
    pub snippet: String,
}

impl From<SearchHit> for SearchHitDto {
    fn from(h: SearchHit) -> Self {
        Self {
            file_id: h.file_id,
            entity_type: h.entity_type,
            name: h.name,
            snippet: h.snippet,
        }
    }
}

/// Run an FTS5 query against the index, optionally intersected
/// with a tag filter (intersection semantics — same as the
/// Python sidecar). Empty `q` short-circuits to an empty list to
/// match the frontend's behaviour and avoid a malformed-MATCH
/// error from FTS5 on empty input.
#[tauri::command]
#[specta::specta]
pub fn core_search(
    state: State<'_, EduportStateHandle>,
    q: String,
    limit: Option<usize>,
    tags: Option<Vec<String>>,
) -> Result<Vec<SearchHitDto>, CommandError> {
    if q.trim().is_empty() {
        return Ok(Vec::new());
    }
    let st = require_state(&state)?;
    let limit = limit.unwrap_or(50).min(500); // upper bound for sanity
    let tags: Vec<&str> = tags
        .as_ref()
        .map(|v| v.iter().map(String::as_str).collect())
        .unwrap_or_default();
    let index = st
        .index
        .lock()
        .map_err(|_| CommandError::internal("index mutex poisoned"))?;
    let hits = search_fts(index.conn(), &q, limit, &tags)?;
    Ok(hits.into_iter().map(Into::into).collect())
}
