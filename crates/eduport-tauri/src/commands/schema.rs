//! Schema editor commands.
//!
//! The schema is owned by `SchemaStore` (atomic YAML writes via
//! vaultdb-core's writer). Mutations no longer trigger a `properties`
//! reindex: filtering moved to `Vault::query`, which reads the on-disk
//! frontmatter directly, so there's no shadow index to keep in sync.
//!
//! `tier_template` and `purge_orphans` are coordinator operations
//! that combine SchemaStore + EntityStore. They live here (rather
//! than as eduport-core methods) because they need both layers, and
//! crossing layers is exactly what the Tauri command surface is for.

use std::path::Path;

use eduport_core::EntityType;
use eduport_core::index::writer::upsert_entity as index_upsert;
use eduport_core::schema::{
    EntitySchema, PatchableFields, Property, PropertyKind, Schema, SchemaStoreError, SelectOption,
    SingleSelectProperty,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tauri::State;

use super::{CommandError, require_state};
use crate::core_state::{EduportState, EduportStateHandle};

impl From<SchemaStoreError> for CommandError {
    fn from(e: SchemaStoreError) -> Self {
        match e {
            SchemaStoreError::Conflict(m) => Self::conflict(m),
            SchemaStoreError::NotFound(m) => Self::not_found(m),
            SchemaStoreError::Invalid(m) => Self::invalid(m),
            SchemaStoreError::Eduport(e) => Self::internal(e.to_string()),
        }
    }
}

/// Patch shape sent by the frontend's `patchProperty`. We accept
/// every known field; absent fields mean "leave alone".
#[derive(Debug, Default, Deserialize)]
pub struct PropertyPatch {
    pub name: Option<String>,
    /// Wrapped option so we can express both "leave alone" (None)
    /// and "clear the description" (Some(None)).
    #[serde(default, deserialize_with = "double_option")]
    pub description: Option<Option<String>>,
    pub required: Option<bool>,
    /// Same double-option treatment for nullable unit.
    #[serde(default, deserialize_with = "double_option")]
    pub unit: Option<Option<String>>,
    pub options: Option<Vec<SelectOption>>,
    /// `target_types: null` means "clear", `target_types: [...]`
    /// means "set"; absence means "leave alone".
    #[serde(default, deserialize_with = "double_option_vec")]
    pub target_types: Option<Option<Vec<EntityType>>>,
}

fn double_option<'de, D, T>(deserialiser: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    Option::<T>::deserialize(deserialiser).map(Some)
}

fn double_option_vec<'de, D, T>(deserialiser: D) -> Result<Option<Option<Vec<T>>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    Option::<Vec<T>>::deserialize(deserialiser).map(Some)
}

#[derive(Debug, Serialize)]
pub struct TierTemplateResult {
    pub results: std::collections::BTreeMap<String, TierStatus>,
    pub schema: Schema,
}

#[derive(Debug, Serialize)]
pub struct TierStatus {
    pub status: &'static str,
}

#[derive(Debug, Serialize)]
pub struct PurgeOrphansResult {
    pub rewritten: usize,
    pub skipped: Vec<PurgeSkip>,
}

#[derive(Debug, Serialize)]
pub struct PurgeSkip {
    pub file_id: String,
    pub reason: String,
}

/// Return the full schema (all eight types + their properties).
#[tauri::command]
pub fn core_schema_get(state: State<'_, EduportStateHandle>) -> Result<Schema, CommandError> {
    let st = require_state(&state)?;
    Ok(st.schema_store.current()?)
}

/// Return one entity-type's portion of the schema.
#[tauri::command]
pub fn core_schema_get_type(
    state: State<'_, EduportStateHandle>,
    entity_type: EntityType,
) -> Result<EntitySchema, CommandError> {
    let st = require_state(&state)?;
    let schema = st.schema_store.current()?;
    Ok(schema.for_type(entity_type).clone())
}

/// Add a property to `entity_type`. Triggers a property reindex so
/// the SQL filter surface picks up the new key on existing entities.
#[tauri::command]
pub fn core_schema_add_property(
    state: State<'_, EduportStateHandle>,
    entity_type: EntityType,
    property: JsonValue,
) -> Result<EntitySchema, CommandError> {
    let st = require_state(&state)?;
    let prop = parse_property(property)?;
    let schema = st.schema_store.add_property(entity_type, prop)?;
    Ok(schema.for_type(entity_type).clone())
}

/// Patch the editable fields of a property. The property's `key`
/// and `type` are immutable post-creation — see
/// `SchemaStore::patch_property` for the full rule set.
#[tauri::command]
pub fn core_schema_patch_property(
    state: State<'_, EduportStateHandle>,
    entity_type: EntityType,
    key: String,
    patch: PropertyPatch,
) -> Result<EntitySchema, CommandError> {
    let st = require_state(&state)?;
    let patchable = PatchableFields {
        name: patch.name,
        description: patch.description,
        required: patch.required,
        unit: patch.unit,
        options: patch.options,
        target_types: patch.target_types,
    };
    let schema = st
        .schema_store
        .patch_property(entity_type, &key, patchable)?;
    Ok(schema.for_type(entity_type).clone())
}

/// Delete a property from the schema. Existing entity values for
/// that key become "orphaned" — they remain on disk; the SQL index
/// drops them (so they no longer appear in filter aggregations) but
/// the YAML keeps them until the user runs `purge_orphans`.
#[tauri::command]
pub fn core_schema_delete_property(
    state: State<'_, EduportStateHandle>,
    entity_type: EntityType,
    key: String,
) -> Result<EntitySchema, CommandError> {
    let st = require_state(&state)?;
    let schema = st.schema_store.delete_property(entity_type, &key)?;
    Ok(schema.for_type(entity_type).clone())
}

/// Reorder the properties of one entity type to match `ordered_keys`.
/// `ordered_keys` must contain exactly the existing keys; otherwise
/// the call errors with `invalid` so the frontend can re-fetch and
/// reconcile UI state.
#[tauri::command]
pub fn core_schema_reorder_properties(
    state: State<'_, EduportStateHandle>,
    entity_type: EntityType,
    ordered_keys: Vec<String>,
) -> Result<EntitySchema, CommandError> {
    let st = require_state(&state)?;
    let schema = st
        .schema_store
        .reorder_properties(entity_type, &ordered_keys)?;
    Ok(schema.for_type(entity_type).clone())
}

/// Apply the built-in "tier" single-select template to each
/// requested entity type. Idempotent: types that already have a
/// `tier` property are reported with status `exists` rather than
/// erroring.
#[tauri::command]
pub fn core_schema_apply_tier_template(
    state: State<'_, EduportStateHandle>,
    types: Vec<EntityType>,
) -> Result<TierTemplateResult, CommandError> {
    let st = require_state(&state)?;
    let mut results = std::collections::BTreeMap::new();
    let mut schema = st.schema_store.current()?;

    for et in types {
        if schema.for_type(et).property("tier").is_some() {
            results.insert(et.to_string(), TierStatus { status: "exists" });
            continue;
        }
        let prop = Property::SingleSelect(SingleSelectProperty {
            key: "tier".into(),
            name: "Tier".into(),
            description: Some("Reach / target / safety bucket".into()),
            required: false,
            options: vec![
                SelectOption {
                    value: "reach".into(),
                    label: "Reach".into(),
                    color: eduport_core::schema::OptionColor::Red,
                },
                SelectOption {
                    value: "target".into(),
                    label: "Target".into(),
                    color: eduport_core::schema::OptionColor::Yellow,
                },
                SelectOption {
                    value: "safety".into(),
                    label: "Safety".into(),
                    color: eduport_core::schema::OptionColor::Green,
                },
            ],
            default: None,
        });
        schema = st.schema_store.add_property(et, prop)?;
        results.insert(et.to_string(), TierStatus { status: "added" });
    }

    Ok(TierTemplateResult { results, schema })
}

/// Rewrite every entity of `entity_type` to drop the orphan key
/// `key`. The schema must NOT currently declare this key (refusal
/// is the only safe behaviour — otherwise we'd be silently deleting
/// live values).
#[tauri::command]
pub fn core_schema_purge_orphans(
    state: State<'_, EduportStateHandle>,
    entity_type: EntityType,
    key: String,
) -> Result<PurgeOrphansResult, CommandError> {
    let st = require_state(&state)?;
    let schema = st.schema_store.current()?;
    if schema.for_type(entity_type).property(&key).is_some() {
        return Err(CommandError::conflict(format!(
            "property {key:?} is currently declared on {entity_type}; \
             delete it from the schema before purging orphans"
        )));
    }

    let entities = st.entity_store.list_by_kind(entity_type)?;
    let mut rewritten = 0usize;
    let mut skipped: Vec<PurgeSkip> = Vec::new();
    let schema_for_index = st.schema_store.current().ok();

    for entity in entities {
        let file_id = match derive_file_id_for_entity(&st, &entity, entity_type) {
            Some(id) => id,
            None => {
                skipped.push(PurgeSkip {
                    file_id: entity.name().to_string(),
                    reason: "could not derive file_id from on-disk path".into(),
                });
                continue;
            }
        };
        let path = st.entity_store.path_for(entity_type, &file_id);
        if !path.exists() {
            skipped.push(PurgeSkip {
                file_id,
                reason: "file missing".into(),
            });
            continue;
        }
        let mut current = entity;
        // The custom-fields BTreeMap on Entity is reachable through
        // `Entity::custom()` (immutable). We need a mutable handle —
        // route through the typed variant struct instead.
        let removed = drop_custom_key(&mut current, &key);
        if !removed {
            // Nothing to do for this file — orphan key wasn't present.
            continue;
        }

        let body = read_body(&path).unwrap_or_default();
        if let Some(watcher) = st.watcher.lock().expect("watcher mutex poisoned").as_ref() {
            watcher.note_self_write(&path);
        }
        if let Err(e) = st
            .entity_store
            .save_with_body(entity_type, &file_id, &current, &body)
        {
            skipped.push(PurgeSkip {
                file_id,
                reason: format!("save failed: {e}"),
            });
            continue;
        }

        // Refresh the index synchronously — same reason as in the
        // entity commands: don't make the user wait for the watcher
        // debounce to see the change.
        let mtime_ns = path
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_nanos() as i64)
            .unwrap_or(0);
        let index = st.index.lock().expect("index mutex poisoned");
        let _ = index_upsert(
            index.conn(),
            &file_id,
            &path,
            mtime_ns,
            &current,
            &body,
            schema_for_index.as_ref(),
        );
        rewritten += 1;
    }

    Ok(PurgeOrphansResult { rewritten, skipped })
}

// ── helpers ────────────────────────────────────────────────────────

/// Parse a JSON property body. Goes through serde_yaml as the
/// canonical wire format (matches what's stored in schema.yaml).
fn parse_property(value: JsonValue) -> Result<Property, CommandError> {
    let yaml: serde_yaml::Value = serde_json::from_value(value)
        .map_err(|e| CommandError::invalid(format!("invalid property body: {e}")))?;
    let yaml_text = serde_yaml::to_string(&yaml)
        .map_err(|e| CommandError::internal(format!("yaml serialise: {e}")))?;
    let prop: Property = serde_yaml::from_str(&yaml_text)
        .map_err(|e| CommandError::invalid(format!("property parse: {e}")))?;
    prop.validate().map_err(CommandError::invalid)?;
    Ok(prop)
}

/// Reverse-derive a `file_id` from an entity by scanning the
/// entity-type folder for a file whose stem produces a parsed
/// entity equal to the given one. Used by purge_orphans, which
/// needs the file_id to call `save_with_body`.
///
/// In the steady state we'd have this from the index, but during a
/// purge we want to operate against the on-disk truth (the index
/// could have a stale file_id if the user just renamed something).
fn derive_file_id_for_entity(
    state: &EduportState,
    entity: &eduport_core::entity::Entity,
    kind: EntityType,
) -> Option<String> {
    // Entity files live flat at the vault root; walk it
    // non-recursively and match on the loaded entity's name.
    let entries = std::fs::read_dir(&state.data_folder).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        let stem = path.file_stem()?.to_str()?.to_string();
        if let Some(loaded) = state.entity_store.find_by_name(kind, &stem).ok().flatten() {
            if loaded.name() == entity.name() {
                return Some(stem);
            }
        }
    }
    None
}

/// Remove a key from the appropriate variant's `custom` map.
/// Returns true if the key was present; false otherwise. Saves us
/// touching every variant inline at the call site.
fn drop_custom_key(entity: &mut eduport_core::entity::Entity, key: &str) -> bool {
    use eduport_core::entity::Entity;
    match entity {
        Entity::University(e) => e.custom.remove(key).is_some(),
        Entity::Lab(e) => e.custom.remove(key).is_some(),
        Entity::Person(e) => e.custom.remove(key).is_some(),
        Entity::Program(e) => e.custom.remove(key).is_some(),
        Entity::Application(e) => e.custom.remove(key).is_some(),
        Entity::Document(e) => e.custom.remove(key).is_some(),
        Entity::Email(e) => e.custom.remove(key).is_some(),
        Entity::Note(e) => e.custom.remove(key).is_some(),
    }
}

fn read_body(path: &Path) -> Option<String> {
    let raw = std::fs::read_to_string(path).ok()?;
    let trimmed = raw.strip_prefix("---\n")?;
    let close = trimmed.find("\n---\n")?;
    Some(trimmed[close + "\n---\n".len()..].to_string())
}

/// Silence unused-warnings for re-exports kept around for future
/// command modules that share this file's type set.
#[allow(dead_code)]
fn _force_use(_: PropertyKind) {}
