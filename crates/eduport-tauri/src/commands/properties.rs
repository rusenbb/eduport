//! Property aggregation + filtering commands.
//!
//! These commands used to call into eduport-core's bespoke
//! `properties` SQLite table + handwritten SQL JOIN builder. That
//! table duplicated work vaultdb-core already does natively — and
//! it was tied to schema-declared keys only, so built-in entity
//! fields (name, country, city, status, …) weren't filterable.
//!
//! The current implementation delegates to `Vault::query` through
//! the `eduport_core::query` adapter. Built-in fields, custom
//! schema fields, and tag intersections all flow through the same
//! `Predicate` AST. No JS post-filter, no shadow index.

use std::collections::BTreeMap;

use eduport_core::query::{query_for_filter, value_counts_for, EntitySummaryView, FilterInput};
use eduport_core::view::FilterTree;
use eduport_core::EntityType;
use serde::{Deserialize, Serialize};
use tauri::State;

use super::{require_state, CommandError};
use crate::commands::entity::EntityListItem;
use crate::core_state::EduportStateHandle;

/// One entry in a property-value-count aggregation. Field-for-field
/// compatible with the frontend's `PropertyCount`.
#[derive(Debug, Serialize)]
pub struct PropertyCount {
    #[serde(rename = "type")]
    pub property_type: String,
    pub value: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct PropertyCountsResponse {
    pub entity_type: EntityType,
    pub key: String,
    pub values: Vec<PropertyCount>,
}

/// Frontend-shaped property filter request. Mirrors what
/// `frontend/src/lib/api/schema.ts:filterEntitiesByProperties`
/// builds. Each map keys a property by name; ranges are
/// `(low, high)` tuples with optional ends.
#[derive(Debug, Default, Deserialize)]
pub struct PropertyFiltersRequest {
    #[serde(default)]
    pub text: BTreeMap<String, String>,
    #[serde(default)]
    pub num: BTreeMap<String, (Option<f64>, Option<f64>)>,
    #[serde(default)]
    pub date: BTreeMap<String, (Option<String>, Option<String>)>,
    /// Optional Notion-style compound filter tree (Phase B). Merged
    /// with the flat chip filter via AND in the query adapter.
    #[serde(default)]
    pub tree: Option<FilterTree>,
    pub sort: Option<String>,
    /// `"asc"` or `"desc"`; anything else is treated as ascending.
    #[serde(default)]
    pub sort_dir: Option<String>,
}

/// Distinct values of `key` over all entities of `entity_type`, with
/// counts. The frontend's property-chip section calls this on mount
/// and on every vault-event refresh.
///
/// vaultdb's Predicate AST doesn't have GROUP BY, so we run a
/// type-pinned `Vault::query` and aggregate in-process. The cost is
/// one disk walk (vaultdb's design — no daemon, no cache) + an O(N)
/// pass over the resulting records. For a vault sized like this app
/// targets (thousands of entities at most), that's noise.
#[tauri::command]
pub fn core_property_value_counts(
    state: State<'_, EduportStateHandle>,
    entity_type: EntityType,
    key: String,
) -> Result<PropertyCountsResponse, CommandError> {
    let st = require_state(&state)?;
    // Empty-input — covers "all entities of this type". The
    // adapter still pins to the type tag.
    let text = BTreeMap::new();
    let num = BTreeMap::new();
    let date = BTreeMap::new();
    let input = FilterInput {
        text: &text,
        num: &num,
        date: &date,
        tags: &[],
        tree: None,
        sort_key: None,
        sort_dir: "asc",
    };
    let q = query_for_filter(entity_type, &input);
    let records = st.entity_store.vault().query(&q).map_err(|e| {
        CommandError::internal(format!("vault.query failed: {e}"))
    })?;
    let pairs = value_counts_for(&records, &key);
    Ok(PropertyCountsResponse {
        entity_type,
        key,
        values: pairs
            .into_iter()
            .map(|(value, count)| PropertyCount {
                // vaultdb doesn't carry the typed-property kind back
                // here; the property store layer is what infers it.
                // The frontend currently only reads `value` + `count`,
                // and the kind is recoverable via the schema store
                // it already loads. Leaving this as the empty string
                // matches what the previous SQL impl produced for
                // unknown-typed property keys.
                property_type: String::new(),
                value,
                count,
            })
            .collect(),
    })
}

/// Filter entities of `entity_type` by the request's text / num /
/// date predicates. Sort and limit handled by vaultdb's `Query`.
#[tauri::command]
pub fn core_filter_entities_by_properties(
    state: State<'_, EduportStateHandle>,
    entity_type: EntityType,
    filters: PropertyFiltersRequest,
) -> Result<Vec<EntityListItem>, CommandError> {
    let st = require_state(&state)?;
    let sort_dir = filters.sort_dir.as_deref().unwrap_or("asc");
    let input = FilterInput {
        text: &filters.text,
        num: &filters.num,
        date: &filters.date,
        tags: &[],
        tree: filters.tree.as_ref(),
        sort_key: filters.sort.as_deref(),
        sort_dir,
    };
    let q = query_for_filter(entity_type, &input);
    let records = st.entity_store.vault().query(&q).map_err(|e| {
        CommandError::internal(format!("vault.query failed: {e}"))
    })?;
    Ok(records
        .iter()
        .filter_map(EntitySummaryView::from_record)
        .map(|s| EntityListItem {
            file_id: s.file_id,
            entity_type: s.entity_type,
            name: s.name,
            path: s.path,
        })
        .collect())
}
