//! Property aggregation + filtering commands.

use std::collections::BTreeMap;

use eduport_core::index::{
    filter_entities_by_properties, property_value_counts, EntitySummary, PropertyFilter,
    PropertyValueCount,
};
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

impl From<PropertyValueCount> for PropertyCount {
    fn from(p: PropertyValueCount) -> Self {
        Self {
            property_type: p.property_type,
            value: p.value,
            count: p.count,
        }
    }
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
    pub sort: Option<String>,
    /// `"asc"` or `"desc"`; anything else is treated as ascending.
    #[serde(default)]
    pub sort_dir: Option<String>,
}

#[tauri::command]
pub fn core_property_value_counts(
    state: State<'_, EduportStateHandle>,
    entity_type: EntityType,
    key: String,
) -> Result<PropertyCountsResponse, CommandError> {
    let st = require_state(&state)?;
    let index = st.index.lock().expect("index mutex poisoned");
    let values = property_value_counts(index.conn(), entity_type, &key)?;
    Ok(PropertyCountsResponse {
        entity_type,
        key,
        values: values.into_iter().map(Into::into).collect(),
    })
}

#[tauri::command]
pub fn core_filter_entities_by_properties(
    state: State<'_, EduportStateHandle>,
    entity_type: EntityType,
    filters: PropertyFiltersRequest,
) -> Result<Vec<EntityListItem>, CommandError> {
    let st = require_state(&state)?;
    // Build the borrowed filter view. We hold the original
    // owned strings here so the borrows stay live for the whole
    // call.
    let text_pairs: Vec<(&str, &str)> = filters
        .text
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    let num_pairs: Vec<(&str, Option<f64>, Option<f64>)> = filters
        .num
        .iter()
        .map(|(k, (lo, hi))| (k.as_str(), *lo, *hi))
        .collect();
    let date_pairs: Vec<(&str, Option<&str>, Option<&str>)> = filters
        .date
        .iter()
        .map(|(k, (lo, hi))| (k.as_str(), lo.as_deref(), hi.as_deref()))
        .collect();
    let sort_dir = filters.sort_dir.as_deref().unwrap_or("asc");
    let pf = PropertyFilter {
        text_equals: &text_pairs,
        num_range: &num_pairs,
        date_range: &date_pairs,
        sort_key: filters.sort.as_deref(),
        sort_dir,
    };

    let index = st.index.lock().expect("index mutex poisoned");
    let rows: Vec<EntitySummary> =
        filter_entities_by_properties(index.conn(), entity_type, &pf)?;
    Ok(rows.into_iter().map(Into::into).collect())
}
