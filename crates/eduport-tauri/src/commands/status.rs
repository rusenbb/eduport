//! Status, counts, tags, and parse-error commands.
//!
//! All read-only queries against the FTS5 index.

use std::collections::BTreeMap;

use eduport_core::EntityType;
use serde::Serialize;
use tauri::State;

use super::{require_state, CommandError};
use crate::core_state::EduportStateHandle;

#[derive(Debug, Serialize)]
pub struct AppStatus {
    pub status: &'static str,
    pub parse_errors: i64,
    pub entities: i64,
}

#[derive(Debug, Serialize)]
pub struct ParseErrorItem {
    pub path: String,
    pub message: String,
    pub occurred_at: String,
}

#[derive(Debug, Serialize)]
pub struct TagCount {
    pub tag: String,
    pub count: i64,
}

/// Whole-app health check. Returns counts the frontend uses to show
/// "N entities, M parse errors" in the status bar.
#[tauri::command]
pub fn core_get_status(state: State<'_, EduportStateHandle>) -> Result<AppStatus, CommandError> {
    let st = require_state(&state)?;
    let index = st.index.lock().expect("index mutex poisoned");
    let entities: i64 = index
        .conn()
        .query_row("SELECT COUNT(*) FROM entities", [], |r| r.get(0))
        .map_err(eduport_core::index::IndexError::from)?;
    let parse_errors: i64 = index
        .conn()
        .query_row("SELECT COUNT(*) FROM parse_errors", [], |r| r.get(0))
        .map_err(eduport_core::index::IndexError::from)?;
    Ok(AppStatus {
        status: "ok",
        parse_errors,
        entities,
    })
}

#[tauri::command]
pub fn core_list_parse_errors(
    state: State<'_, EduportStateHandle>,
) -> Result<Vec<ParseErrorItem>, CommandError> {
    let st = require_state(&state)?;
    let index = st.index.lock().expect("index mutex poisoned");
    let mut stmt = index
        .conn()
        .prepare(
            "SELECT path, message, occurred_at FROM parse_errors \
             ORDER BY occurred_at DESC, path ASC",
        )
        .map_err(eduport_core::index::IndexError::from)?;
    let rows = stmt
        .query_map([], |r| {
            Ok(ParseErrorItem {
                path: r.get(0)?,
                message: r.get(1)?,
                occurred_at: r.get(2)?,
            })
        })
        .map_err(eduport_core::index::IndexError::from)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(eduport_core::index::IndexError::from)?);
    }
    Ok(out)
}

#[tauri::command]
pub fn core_get_counts(
    state: State<'_, EduportStateHandle>,
) -> Result<BTreeMap<String, i64>, CommandError> {
    let st = require_state(&state)?;
    let index = st.index.lock().expect("index mutex poisoned");
    let mut stmt = index
        .conn()
        .prepare("SELECT type, COUNT(*) FROM entities GROUP BY type")
        .map_err(eduport_core::index::IndexError::from)?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))
        .map_err(eduport_core::index::IndexError::from)?;

    // Seed every entity type at 0 so the frontend doesn't have to
    // fall back when one type has no entities (matches the Python
    // sidecar's behaviour).
    let mut out: BTreeMap<String, i64> = BTreeMap::new();
    for t in EntityType::ALL {
        out.insert(t.to_string(), 0);
    }
    for row in rows {
        let (t, c) = row.map_err(eduport_core::index::IndexError::from)?;
        out.insert(t, c);
    }
    Ok(out)
}

/// All tags across the vault, with counts. The `eduport-type/...`
/// internal discriminator tags are suppressed — the frontend
/// surfaces only user-meaningful tags.
#[tauri::command]
pub fn core_get_tags(state: State<'_, EduportStateHandle>) -> Result<Vec<TagCount>, CommandError> {
    let st = require_state(&state)?;
    let index = st.index.lock().expect("index mutex poisoned");
    let mut stmt = index
        .conn()
        .prepare(
            "SELECT tag, COUNT(*) FROM entity_tags \
             WHERE tag NOT LIKE 'eduport-type/%' \
             GROUP BY tag ORDER BY tag",
        )
        .map_err(eduport_core::index::IndexError::from)?;
    let rows = stmt
        .query_map([], |r| {
            Ok(TagCount {
                tag: r.get(0)?,
                count: r.get(1)?,
            })
        })
        .map_err(eduport_core::index::IndexError::from)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(eduport_core::index::IndexError::from)?);
    }
    Ok(out)
}
