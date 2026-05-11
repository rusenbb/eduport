//! Entity CRUD commands.
//!
//! Mirrors the Python sidecar's `/entities/*` endpoints. Each handler
//! is a thin shim over `EntityStore` + the FTS5 index — the heavy
//! lifting lives in `eduport-core`.
//!
//! ## Notify-self-write integration
//!
//! Every mutating handler (`create`, `update`, `delete`) calls
//! `Watcher::note_self_write` before the on-disk write, so the
//! watcher's debouncer doesn't bounce the file back through the
//! parse path. Without this, `create_entity` would round-trip
//! through the watcher and re-index a file we already have in
//! memory.

use std::path::Path;
use std::sync::Arc;

use eduport_core::entity::Entity;
use eduport_core::index::writer::{delete_entity as index_delete, upsert_entity as index_upsert};
use eduport_core::query::{query_for_children, query_for_filter, EntitySummaryView, FilterInput};
use eduport_core::EntityType;
use serde::Serialize;
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use tauri::State;

use super::{require_state, CommandError};
use crate::core_state::{EduportState, EduportStateHandle};

/// One row in the entity-list view. Field-for-field compatible with
/// the frontend's `EntityListItem`.
#[derive(Debug, Serialize)]
pub struct EntityListItem {
    pub file_id: String,
    #[serde(rename = "type")]
    pub entity_type: String,
    pub name: String,
    pub path: String,
}

impl From<EntitySummaryView> for EntityListItem {
    fn from(s: EntitySummaryView) -> Self {
        Self {
            file_id: s.file_id,
            entity_type: s.entity_type,
            name: s.name,
            path: s.path,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Backlink {
    pub src_file_id: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct EntityDetail {
    pub file_id: String,
    #[serde(rename = "type")]
    pub entity_type: String,
    pub path: String,
    /// Full frontmatter as a serde_json `Value` so the frontend gets
    /// the typed entity with its custom-property tail. Same shape
    /// the sidecar's GET /entities/{type}/{file_id} returned.
    pub entity: JsonValue,
    pub body: String,
    pub backlinks: Vec<Backlink>,
}

#[derive(Debug, Serialize)]
pub struct CreateResult {
    pub file_id: String,
}

#[derive(Debug, Serialize)]
pub struct ResolveResult {
    pub file_id: String,
    #[serde(rename = "type")]
    pub entity_type: String,
    pub name: String,
}

/// List entities of `entity_type`, optionally filtered by tags
/// (intersection semantics). Delegates to vaultdb's `Vault::query`
/// via the `eduport_core::query` adapter — every tag becomes a
/// `Predicate::Contains { field: "tags", ... }` clause, pinned by
/// the `eduport-type/<value>` discriminator. No SQLite cache lookup.
#[tauri::command]
pub fn core_entity_list(
    state: State<'_, EduportStateHandle>,
    entity_type: EntityType,
    tags: Option<Vec<String>>,
) -> Result<Vec<EntityListItem>, CommandError> {
    let st = require_state(&state)?;
    let tag_strs: Vec<String> = tags.unwrap_or_default();
    let tag_refs: Vec<&str> = tag_strs.iter().map(String::as_str).collect();
    let text = BTreeMap::new();
    let num = BTreeMap::new();
    let date = BTreeMap::new();
    let input = FilterInput {
        text: &text,
        num: &num,
        date: &date,
        tags: &tag_refs,
        tree: None,
        sort_key: None,
        sort_dir: "asc",
    };
    let q = query_for_filter(entity_type, &input);
    let records = st
        .entity_store
        .vault()
        .query(&q)
        .map_err(|e| CommandError::internal(format!("vault.query failed: {e}")))?;
    Ok(records
        .iter()
        .filter_map(EntitySummaryView::from_record)
        .map(Into::into)
        .collect())
}

/// List entities whose `parent` frontmatter field equals `parent_file_id`.
/// Cross-type — a Person can be a sub-page of a University, a Note can
/// be a sub-page of an Application, etc. Backs the page-hierarchy
/// "sub-pages" UI in DetailPanel.
///
/// Returns a flat list sorted by `name`. The frontend handles the
/// tree shape (this is one fetch per node, lazy on expand).
#[tauri::command]
pub fn core_entity_children(
    state: State<'_, EduportStateHandle>,
    parent_file_id: String,
) -> Result<Vec<EntityListItem>, CommandError> {
    let st = require_state(&state)?;
    let q = query_for_children(&parent_file_id);
    let records = st
        .entity_store
        .vault()
        .query(&q)
        .map_err(|e| CommandError::internal(format!("vault.query failed: {e}")))?;
    Ok(records
        .iter()
        .filter_map(EntitySummaryView::from_record)
        .map(Into::into)
        .collect())
}

/// Fetch a single entity by `(entity_type, file_id)`. Reads the
/// canonical file via `EntityStore::find_by_name` (same parser as
/// list / reconcile), assembles backlinks from vaultdb's link
/// graph, and emits the full detail payload.
#[tauri::command]
pub fn core_entity_get(
    state: State<'_, EduportStateHandle>,
    entity_type: EntityType,
    file_id: String,
) -> Result<EntityDetail, CommandError> {
    let st = require_state(&state)?;
    let entity_opt = st
        .entity_store
        .find_by_name(entity_type, &file_id)
        .map_err(CommandError::from)?;
    let entity = entity_opt
        .ok_or_else(|| CommandError::not_found(format!("no entity {entity_type}/{file_id}")))?;
    let path = st.entity_store.path_for(entity_type, &file_id);
    let body = read_body(&path).unwrap_or_default();
    let entity_name = entity.name().to_string();
    let entity_json = entity_to_json(&entity)?;
    // LinkGraph keys by entity name, not file_id — see
    // collect_backlinks doc comment for why.
    let backlinks = collect_backlinks(&st, &entity_name);
    Ok(EntityDetail {
        file_id,
        entity_type: entity_type.to_string(),
        path: path.to_string_lossy().into_owned(),
        entity: entity_json,
        body,
        backlinks,
    })
}

/// Resolve a wikilink target (e.g. `Stanford`) to a single entity.
/// Errors if the target is ambiguous (matches more than one) or
/// missing. Mirrors the sidecar's `/entities/resolve/{target}`.
#[tauri::command]
pub fn core_entity_resolve(
    state: State<'_, EduportStateHandle>,
    target: String,
) -> Result<ResolveResult, CommandError> {
    let st = require_state(&state)?;
    let index = st
        .index
        .lock()
        .map_err(|_| CommandError::internal("index mutex poisoned"))?;
    // Match against either the file_id (filename stem) or the entity
    // name. Type comes from the FTS row's joined tags column —
    // `eduport-type/<value>` is the discriminator. The bespoke
    // `entities.type` SQL column went away when storage moved to the
    // shared vaultdb-fts crate (which is type-agnostic on purpose).
    let mut matches: Vec<(String, String, String)> = Vec::new();
    let mut stmt = index
        .conn()
        .prepare(
            "SELECT e.file_id, e.name, ef.tags \
             FROM entities e \
             JOIN entities_fts ef ON e.rowid = ef.rowid \
             WHERE e.file_id = ?1 OR e.name = ?1",
        )
        .map_err(eduport_core::index::IndexError::from)?;
    let mut rows = stmt
        .query([&target])
        .map_err(eduport_core::index::IndexError::from)?;
    while let Some(row) = rows.next().map_err(eduport_core::index::IndexError::from)? {
        let file_id: String = row.get(0).map_err(eduport_core::index::IndexError::from)?;
        let name: String = row.get(1).map_err(eduport_core::index::IndexError::from)?;
        let tags: String = row.get(2).map_err(eduport_core::index::IndexError::from)?;
        let Some(entity_type) = tags
            .split_whitespace()
            .find_map(|t| t.strip_prefix("eduport-type/").map(String::from))
        else {
            continue;
        };
        matches.push((file_id, entity_type, name));
    }
    drop(rows);
    drop(stmt);

    match matches.len() {
        0 => Err(CommandError::not_found(format!(
            "no entity matching wikilink target {target:?}"
        ))),
        1 => {
            let (file_id, entity_type, name) = matches.remove(0);
            Ok(ResolveResult {
                file_id,
                entity_type,
                name,
            })
        }
        n => Err(CommandError::conflict(format!(
            "wikilink target {target:?} is ambiguous: {n} matches"
        ))),
    }
}

/// Create a new entity. The `frontmatter` is an arbitrary JSON
/// object; we serialise it through serde_yaml to match the on-disk
/// YAML and parse it as an `Entity`. The file's body is whatever
/// `body` carries.
#[tauri::command]
pub fn core_entity_create(
    state: State<'_, EduportStateHandle>,
    entity_type: EntityType,
    frontmatter: JsonValue,
    body: Option<String>,
) -> Result<CreateResult, CommandError> {
    let st = require_state(&state)?;
    let body = body.unwrap_or_default();
    let entity = json_to_entity(frontmatter, entity_type)?;
    let file_id = generate_unique_file_id(&st, entity_type, entity.name())?;
    let path = st.entity_store.path_for(entity_type, &file_id);

    note_self_write(&st, &path);
    st.entity_store
        .create(entity_type, &file_id, &entity, &body)?;

    // Update the index synchronously — the watcher's debounce
    // window means the index would lag the user-visible action by
    // up to 200 ms otherwise.
    upsert_into_index(&st, &file_id, &path, &entity, &body)?;

    Ok(CreateResult { file_id })
}

/// Update an existing entity (PATCH semantics — the full new
/// frontmatter and body replace the previous ones).
#[tauri::command]
pub fn core_entity_update(
    state: State<'_, EduportStateHandle>,
    entity_type: EntityType,
    file_id: String,
    frontmatter: JsonValue,
    body: Option<String>,
) -> Result<CreateResult, CommandError> {
    let st = require_state(&state)?;
    let body = body.unwrap_or_default();
    let entity = json_to_entity(frontmatter, entity_type)?;
    let path = st.entity_store.path_for(entity_type, &file_id);

    note_self_write(&st, &path);
    st.entity_store
        .save_with_body(entity_type, &file_id, &entity, &body)?;
    upsert_into_index(&st, &file_id, &path, &entity, &body)?;

    Ok(CreateResult { file_id })
}

/// Delete an entity. Routes through `EntityStore::delete(.., false)`
/// which moves the file to vaultdb's `.trash/` rather than removing
/// it outright (collision-safe). The trash commands (Phase 9.5)
/// expose list/restore/empty.
#[tauri::command]
pub fn core_entity_delete(
    state: State<'_, EduportStateHandle>,
    entity_type: EntityType,
    file_id: String,
) -> Result<(), CommandError> {
    let st = require_state(&state)?;
    let path = st.entity_store.path_for(entity_type, &file_id);
    note_self_write(&st, &path);
    st.entity_store.delete(entity_type, &file_id, false)?;
    let index = st
        .index
        .lock()
        .map_err(|_| CommandError::internal("index mutex poisoned"))?;
    index_delete(index.conn(), &file_id)?;
    Ok(())
}

// ── Helpers ─────────────────────────────────────────────────────

/// Convert an [`Entity`] back into a serde_json::Value for the
/// frontend. Direct serde_json dispatch per variant — no YAML round
/// trip, since the typed structs now flatten their custom-property
/// tail into `serde_json::Value` natively.
fn entity_to_json(entity: &Entity) -> Result<JsonValue, CommandError> {
    let to_value = |r: Result<JsonValue, serde_json::Error>| {
        r.map_err(|e| CommandError::internal(format!("entity serialise: {e}")))
    };
    match entity {
        Entity::University(e) => to_value(serde_json::to_value(e)),
        Entity::Lab(e) => to_value(serde_json::to_value(e)),
        Entity::Person(e) => to_value(serde_json::to_value(e)),
        Entity::Program(e) => to_value(serde_json::to_value(e)),
        Entity::Application(e) => to_value(serde_json::to_value(e)),
        Entity::Document(e) => to_value(serde_json::to_value(e)),
        Entity::Email(e) => to_value(serde_json::to_value(e)),
        Entity::Note(e) => to_value(serde_json::to_value(e)),
    }
}

fn json_to_entity(frontmatter: JsonValue, expected: EntityType) -> Result<Entity, CommandError> {
    // Look at the embedded `eduport-type/<x>` tag and compare to the
    // command's `expected` type before dispatching deserialisation —
    // catches a buggy frontend that sends a payload-vs-route mismatch.
    let actual = json_eduport_type(&frontmatter);
    if actual != Some(expected) {
        return Err(CommandError::invalid(format!(
            "frontmatter declares {} but command targets {}",
            actual
                .map(|t| t.to_string())
                .unwrap_or_else(|| "no entity type".into()),
            expected,
        )));
    }
    fn invalid_shape(e: serde_json::Error) -> CommandError {
        CommandError::invalid(format!("invalid frontmatter shape: {e}"))
    }
    Ok(match expected {
        EntityType::University => {
            Entity::University(serde_json::from_value(frontmatter).map_err(invalid_shape)?)
        }
        EntityType::Lab => Entity::Lab(serde_json::from_value(frontmatter).map_err(invalid_shape)?),
        EntityType::Person => {
            Entity::Person(serde_json::from_value(frontmatter).map_err(invalid_shape)?)
        }
        EntityType::Program => {
            Entity::Program(serde_json::from_value(frontmatter).map_err(invalid_shape)?)
        }
        EntityType::Application => {
            Entity::Application(serde_json::from_value(frontmatter).map_err(invalid_shape)?)
        }
        EntityType::Document => {
            Entity::Document(serde_json::from_value(frontmatter).map_err(invalid_shape)?)
        }
        EntityType::Email => {
            Entity::Email(serde_json::from_value(frontmatter).map_err(invalid_shape)?)
        }
        EntityType::Note => {
            Entity::Note(serde_json::from_value(frontmatter).map_err(invalid_shape)?)
        }
    })
}

fn json_eduport_type(json: &JsonValue) -> Option<EntityType> {
    let tags = json.get("tags")?.as_array()?;
    for v in tags {
        let s = v.as_str()?;
        if let Some(rest) = s.strip_prefix(eduport_core::entity::EDUPORT_TYPE_PREFIX) {
            return rest.parse::<EntityType>().ok();
        }
    }
    None
}

/// Generate a fresh `file_id` for a new entity. Uses the same shape
/// as `EntityStore`'s on-disk convention: slugify the name, append
/// a 4-char ID, retry on collision (the retry happens inside
/// `eduport_core::generate_id` via the supplied predicate).
fn generate_unique_file_id(
    state: &EduportState,
    entity_type: EntityType,
    name: &str,
) -> Result<String, CommandError> {
    let slug = eduport_core::generate_slug(name);
    let id = eduport_core::generate_id(|candidate| {
        let probe = if slug.is_empty() {
            candidate.to_string()
        } else {
            format!("{slug}-{candidate}")
        };
        state.entity_store.path_for(entity_type, &probe).exists()
    })
    .ok_or_else(|| {
        CommandError::conflict("could not generate a unique file_id; vault may be saturated")
    })?;
    Ok(if slug.is_empty() {
        id
    } else {
        format!("{slug}-{id}")
    })
}

fn note_self_write(state: &EduportState, path: &Path) {
    // Poison would mean a panicked watcher-mutating thread. The write
    // itself isn't blocked; we just skip the debounce nudge and let
    // the watcher pick the file up on its normal scan.
    if let Ok(guard) = state.watcher.lock()
        && let Some(watcher) = guard.as_ref()
    {
        watcher.note_self_write(path);
    }
}

fn upsert_into_index(
    state: &EduportState,
    file_id: &str,
    path: &Path,
    entity: &Entity,
    body: &str,
) -> Result<(), CommandError> {
    let mtime_ns = path
        .metadata()
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0);
    let schema = state.schema_store.current().ok();
    let index = state
        .index
        .lock()
        .map_err(|_| CommandError::internal("index mutex poisoned"))?;
    index_upsert(
        index.conn(),
        file_id,
        path,
        mtime_ns,
        entity,
        body,
        schema.as_ref(),
    )?;
    Ok(())
}

fn read_body(path: &Path) -> Option<String> {
    let raw = std::fs::read_to_string(path).ok()?;
    let trimmed = raw.strip_prefix("---\n")?;
    let close = trimmed.find("\n---\n")?;
    Some(trimmed[close + "\n---\n".len()..].to_string())
}

/// Collect backlinks for an entity from vaultdb's link graph.
///
/// Notes on the data shape:
/// - vaultdb's `LinkGraph` keys by note *name* (e.g. "Stanford
///   University"), not file_id. We pass `name` as the lookup key.
/// - The graph doesn't preserve which frontmatter *field* a wikilink
///   came from (the field column existed in the Python sidecar's
///   `entity_links` table; we deliberately dropped that table in
///   Phase 7 — see crate::index::schema). So `field` is left empty
///   here. If a future surface needs field tracking we'll either
///   teach `LinkGraph` to retain it, or maintain a parallel index
///   for the field-aware backlink path.
/// - Empty Vec when the graph reports nothing — never an error
///   path, because a missing graph would have failed at boot.
fn collect_backlinks(state: &Arc<EduportState>, name: &str) -> Vec<Backlink> {
    let vault = state.entity_store.vault();
    let graph = match vault.link_graph(eduport_core::GraphScope::All) {
        Ok(g) => g,
        Err(_) => return Vec::new(),
    };
    let incoming: Vec<String> = graph
        .incoming_links(name)
        .into_iter()
        .map(str::to_string)
        .collect();
    if incoming.is_empty() {
        return Vec::new();
    }
    // Build a one-shot {file_stem → entity_type} lookup by scanning
    // the vault root once. Previously this was per-backlink SQL
    // against the FTS `entities.type` column — which doesn't exist on
    // the shared vaultdb-fts schema (the column was dropped when
    // storage moved off the bespoke table), so the SQL silently
    // returned None for every source. vaultdb's link graph is built
    // from the same vault scan, so the cost is amortised.
    let kind_by_name: std::collections::HashMap<String, String> = vault
        .query(&vaultdb_core::Query {
            folder: String::new(),
            filter: None,
            select: None,
            sort: None,
            limit: None,
            recursive: false,
        })
        .unwrap_or_default()
        .into_iter()
        .filter_map(|r| {
            let stem = r.path.file_stem()?.to_str()?.to_string();
            let kind = eduport_core::entity::record_eduport_type(&r)?;
            Some((stem, kind.to_string()))
        })
        .collect();
    incoming
        .into_iter()
        .map(|src_name| {
            let entity_type = kind_by_name.get(&src_name).cloned();
            Backlink {
                src_file_id: src_name.clone(),
                entity_type,
                name: Some(src_name),
            }
        })
        .collect()
}
