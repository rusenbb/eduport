//! Status, counts, tags, and parse-error commands.
//!
//! `core_get_counts` and `core_get_tags` go through `vault.query()` —
//! the on-disk markdown is the canonical source of truth. The FTS5
//! cache is for full-text search, not for aggregation. Going through
//! vaultdb keeps these queries correct even when the cache is stale
//! or being rebuilt; it also matches the substrate-correct pattern
//! the rest of `commands/` already uses.

use std::collections::BTreeMap;

use eduport_core::EntityType;
use eduport_core::entity::types::EDUPORT_TYPE_PREFIX;
use serde::Serialize;
use tauri::State;
use vaultdb_core::{Query, Value};

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
    let records = root_records(&st)?;
    let entities = records.iter().filter(|r| is_entity(r)).count() as i64;

    let index = st
        .index
        .lock()
        .map_err(|_| CommandError::internal("index mutex poisoned"))?;
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
    let index = st
        .index
        .lock()
        .map_err(|_| CommandError::internal("index mutex poisoned"))?;
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

/// Counts per entity type. Walks the vault root once and buckets by
/// the `eduport-type/<value>` discriminator tag — one query, no FTS
/// dependency.
#[tauri::command]
pub fn core_get_counts(
    state: State<'_, EduportStateHandle>,
) -> Result<BTreeMap<String, i64>, CommandError> {
    let st = require_state(&state)?;
    let records = root_records(&st)?;

    let mut out: BTreeMap<String, i64> = BTreeMap::new();
    for t in EntityType::ALL {
        out.insert(t.to_string(), 0);
    }
    for record in &records {
        if let Some(Value::List(items)) = record.fields.get("tags") {
            for v in items {
                if let Value::String(s) = v
                    && let Some(kind) = s.strip_prefix(EDUPORT_TYPE_PREFIX)
                {
                    *out.entry(kind.to_string()).or_insert(0) += 1;
                    break;
                }
            }
        }
    }
    Ok(out)
}

/// All user-visible tags across the vault, with counts. Internal
/// discriminator tags (`eduport-type/...` and `eduport-doctype/...`)
/// are suppressed.
#[tauri::command]
pub fn core_get_tags(state: State<'_, EduportStateHandle>) -> Result<Vec<TagCount>, CommandError> {
    let st = require_state(&state)?;
    let records = root_records(&st)?;

    let mut buckets: BTreeMap<String, i64> = BTreeMap::new();
    for record in &records {
        if let Some(Value::List(items)) = record.fields.get("tags") {
            for v in items {
                if let Value::String(s) = v {
                    if s.starts_with(EDUPORT_TYPE_PREFIX) || s.starts_with("eduport-doctype/") {
                        continue;
                    }
                    *buckets.entry(s.clone()).or_insert(0) += 1;
                }
            }
        }
    }
    let mut out: Vec<TagCount> = buckets
        .into_iter()
        .map(|(tag, count)| TagCount { tag, count })
        .collect();
    out.sort_by(|a, b| a.tag.cmp(&b.tag));
    Ok(out)
}

/// Load all top-level vault records non-recursively. Used by every
/// aggregation command in this module so subfolder content
/// (`notes/`, `attachments/`) doesn't leak into the type/tag tallies.
fn root_records(
    st: &crate::core_state::EduportState,
) -> Result<Vec<vaultdb_core::Record>, CommandError> {
    let q = Query {
        folder: String::new(),
        filter: None,
        select: None,
        sort: None,
        limit: None,
        recursive: false,
    };
    st.entity_store
        .vault()
        .query(&q)
        .map_err(|e| CommandError::internal(format!("vault.query failed: {e}")))
}

/// True iff the record carries an `eduport-type/<value>` tag. Used to
/// distinguish entity files from user-managed plain markdown at the
/// vault root.
fn is_entity(record: &vaultdb_core::Record) -> bool {
    if let Some(Value::List(items)) = record.fields.get("tags") {
        items.iter().any(|v| match v {
            Value::String(s) => s.starts_with(EDUPORT_TYPE_PREFIX),
            _ => false,
        })
    } else {
        false
    }
}
