//! Saved-views commands.
//!
//! Mirrors the Python sidecar's `/api/views/*` endpoints. The
//! ViewStore handles validation + atomic writes; these handlers
//! generate fresh ids on create (matching the Python sidecar's
//! convention of slugifying the user-provided name) and forward
//! everything else verbatim.

use eduport_core::view::store::ViewStoreError;
use eduport_core::view::types::{SortDir, TypeViews, View, ViewFilter, ViewKind, ViewsFile};
use eduport_core::EntityType;
use serde::{Deserialize, Serialize};
use tauri::State;

use super::{require_state, CommandError};
use crate::core_state::EduportStateHandle;

impl From<ViewStoreError> for CommandError {
    fn from(e: ViewStoreError) -> Self {
        match e {
            ViewStoreError::Conflict(m) => Self::conflict(m),
            ViewStoreError::NotFound(m) => Self::not_found(m),
            ViewStoreError::Invalid(m) => Self::invalid(m),
            ViewStoreError::Eduport(e) => Self::internal(e.to_string()),
        }
    }
}

/// Create-view body. Same shape the frontend's `CreateViewBody`
/// sends. `kind`/`sort_dir` use the same `lowercase` variants the
/// types crate exposes; absent fields take struct defaults.
#[derive(Debug, Default, Deserialize)]
pub struct CreateViewBody {
    pub name: String,
    #[serde(default)]
    pub kind: ViewKind,
    #[serde(default)]
    pub filter: ViewFilter,
    /// Notion-style compound filter (Phase B). `None` keeps the saved
    /// view chip-only, exactly like before.
    #[serde(default)]
    pub filter_tree: Option<eduport_core::view::FilterTree>,
    pub sort_key: Option<String>,
    #[serde(default)]
    pub sort_dir: SortDir,
    pub group_by_key: Option<String>,
    pub columns: Option<Vec<String>>,
    pub card_properties: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateViewBody {
    pub name: String,
    pub kind: ViewKind,
    pub filter: ViewFilter,
    #[serde(default)]
    pub filter_tree: Option<eduport_core::view::FilterTree>,
    pub sort_key: Option<String>,
    #[serde(default)]
    pub sort_dir: SortDir,
    pub group_by_key: Option<String>,
    pub columns: Option<Vec<String>>,
    pub card_properties: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct ViewMutationResult {
    pub view: View,
    pub type_views: TypeViews,
}

#[tauri::command]
pub fn core_view_get_all(state: State<'_, EduportStateHandle>) -> Result<ViewsFile, CommandError> {
    let st = require_state(&state)?;
    Ok(st.view_store.current()?)
}

#[tauri::command]
pub fn core_view_get_for_type(
    state: State<'_, EduportStateHandle>,
    entity_type: EntityType,
) -> Result<TypeViews, CommandError> {
    let st = require_state(&state)?;
    let file = st.view_store.current()?;
    Ok(file.for_type(entity_type).clone())
}

#[tauri::command]
pub fn core_view_create(
    state: State<'_, EduportStateHandle>,
    entity_type: EntityType,
    body: CreateViewBody,
) -> Result<ViewMutationResult, CommandError> {
    let st = require_state(&state)?;
    let id = generate_view_id(&st.view_store.current()?, entity_type, &body.name);
    let view = View {
        id,
        name: body.name,
        kind: body.kind,
        filter: body.filter,
        filter_tree: body.filter_tree,
        sort_key: body.sort_key,
        sort_dir: body.sort_dir,
        group_by_key: body.group_by_key,
        columns: body.columns,
        card_properties: body.card_properties,
    };
    let file = st.view_store.add_view(entity_type, view.clone())?;
    Ok(ViewMutationResult {
        view,
        type_views: file.for_type(entity_type).clone(),
    })
}

#[tauri::command]
pub fn core_view_update(
    state: State<'_, EduportStateHandle>,
    entity_type: EntityType,
    view_id: String,
    body: UpdateViewBody,
) -> Result<ViewMutationResult, CommandError> {
    let st = require_state(&state)?;
    let view = View {
        id: view_id,
        name: body.name,
        kind: body.kind,
        filter: body.filter,
        filter_tree: body.filter_tree,
        sort_key: body.sort_key,
        sort_dir: body.sort_dir,
        group_by_key: body.group_by_key,
        columns: body.columns,
        card_properties: body.card_properties,
    };
    let file = st.view_store.update_view(entity_type, view.clone())?;
    Ok(ViewMutationResult {
        view,
        type_views: file.for_type(entity_type).clone(),
    })
}

#[tauri::command]
pub fn core_view_delete(
    state: State<'_, EduportStateHandle>,
    entity_type: EntityType,
    view_id: String,
) -> Result<TypeViews, CommandError> {
    let st = require_state(&state)?;
    let file = st.view_store.delete_view(entity_type, &view_id)?;
    Ok(file.for_type(entity_type).clone())
}

#[tauri::command]
pub fn core_view_reorder(
    state: State<'_, EduportStateHandle>,
    entity_type: EntityType,
    ordered_ids: Vec<String>,
) -> Result<TypeViews, CommandError> {
    let st = require_state(&state)?;
    let file = st.view_store.reorder_views(entity_type, &ordered_ids)?;
    Ok(file.for_type(entity_type).clone())
}

/// Generate a fresh view id by slugifying the name and appending a
/// disambiguator if needed. Matches what the Python sidecar did, so
/// existing on-disk views.yaml files don't need rewriting.
fn generate_view_id(file: &ViewsFile, entity_type: EntityType, name: &str) -> String {
    let base = eduport_core::generate_slug(name);
    let base = if base.is_empty() {
        "view".to_string()
    } else {
        base
    };
    let existing: std::collections::HashSet<&str> = file
        .for_type(entity_type)
        .views
        .iter()
        .map(|v| v.id.as_str())
        .collect();
    if !existing.contains(base.as_str()) {
        return base;
    }
    for n in 2..1024 {
        let candidate = format!("{base}-{n}");
        if !existing.contains(candidate.as_str()) {
            return candidate;
        }
    }
    // Pathological — fall through to a random suffix. eduport-core's
    // generate_id ignores its predicate when it can't satisfy uniqueness;
    // we use a fresh unconditionally-unique id instead.
    let id = eduport_core::generate_id(|_| false).expect("generate_id with non-existing predicate");
    format!("{base}-{id}")
}
