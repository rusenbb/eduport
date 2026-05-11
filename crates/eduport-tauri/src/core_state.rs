//! In-process eduport-core state managed by Tauri.
//!
//! This module owns the Rust replacement for what the Python sidecar
//! used to provide over HTTP: a vault, an entity store, a schema
//! store, a view store, an FTS5 index, and a file watcher. Phases 10
//! and 11 remove the sidecar entirely; this state struct is the
//! single source of truth from then on.
//!
//! ## Lifetime model
//!
//! The state is created lazily on first request (`ensure_started`)
//! using the user-provided settings file. We don't try to reconstruct
//! it on every command — that would mean re-walking the vault for a
//! one-line query. Instead the state lives in
//! [`tauri::State<EduportStateHandle>`] and is rebuilt on settings
//! change (which also reboots the watcher).
//!
//! ## Locking
//!
//! `Connection` is `!Sync`, so the index is held behind a `Mutex`.
//! Eduport's index workload is dominated by very fast queries (FTS5
//! on 1k entities is sub-millisecond), so `Mutex` rather than
//! `RwLock` is the right call: the latter would buy us nothing for
//! `rusqlite` (which can't share a connection across threads anyway)
//! and complicates the writer path.
//!
//! ## Error model
//!
//! Boot errors collapse to [`BootError`]; command errors are
//! defined per-module under `crate::commands::*`. We keep
//! eduport-core's specific error types reachable through `BootError`
//! so the caller can render whichever message is most useful (e.g.
//! "schema validation failed" vs "I/O error").

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use eduport_core::entity::EntityStore;
use eduport_core::index::{self, Index};
use eduport_core::schema::SchemaStore;
use eduport_core::view::store::ViewStore;
use eduport_core::watcher::{DEFAULT_DEBOUNCE, VaultEvent, Watcher};
use eduport_core::{Settings, Vault};
use tauri::Emitter;

/// Live application state used by every Tauri command in the
/// `commands::*` modules.
pub struct EduportState {
    pub data_folder: PathBuf,
    pub user_email: String,
    pub entity_store: EntityStore,
    pub schema_store: SchemaStore,
    pub view_store: ViewStore,
    /// SQLite + FTS5 index. Held behind a `Mutex<Index>` because
    /// rusqlite's `Connection` is `!Sync` and the index is mutated
    /// from both command handlers and the watcher worker. We hold
    /// the wrapper [`Index`] (not the raw `Connection`) so callers
    /// don't need a direct rusqlite dependency.
    pub index: Mutex<Index>,
    /// File watcher handle. `None` until `start_watcher` runs;
    /// dropping it stops the watcher threads.
    pub watcher: Mutex<Option<Watcher>>,
}

/// Tauri-managed handle. The outer `Mutex<Option<...>>` lets us
/// rebuild the inner state when the user changes their data folder
/// from the settings UI: take the old handle, drop it (which stops
/// the watcher), build a fresh one, swap it in.
pub type EduportStateHandle = Mutex<Option<Arc<EduportState>>>;

#[derive(Debug, thiserror::Error)]
pub enum BootError {
    #[error(transparent)]
    Eduport(#[from] eduport_core::EduportError),
    #[error(transparent)]
    Index(#[from] eduport_core::index::IndexError),
    #[error(transparent)]
    Watcher(#[from] eduport_core::watcher::WatcherError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Build a fresh [`EduportState`] from a `Settings` value. Opens (or
/// creates) the FTS5 index, runs a reconcile to bring it into
/// agreement with the on-disk vault, and starts the watcher.
///
/// The watcher's callback emits Tauri events to the `main` window.
/// Phase 10's frontend listens for those to refresh views without
/// the user reloading.
pub fn build_state(
    app_handle: &tauri::AppHandle,
    settings: &Settings,
) -> Result<Arc<EduportState>, BootError> {
    let data_folder = PathBuf::from(&settings.data_folder);
    if !data_folder.exists() {
        std::fs::create_dir_all(&data_folder)?;
    }

    // The vault root is the data folder. EntityStore owns the Vault.
    // Entity files live flat at the root, discriminated by their
    // `eduport-type/<value>` tag — no per-type subfolders.
    let vault = Vault::with_root(data_folder.clone());
    let entity_store = EntityStore::new(vault);
    let schema_store = SchemaStore::new(data_folder.clone());
    let view_store = ViewStore::new(data_folder.clone());

    // Open the index file under .eduport/. Use a stable filename so
    // a settings.toml move doesn't lose the cache.
    let ed_dir = data_folder.join(".eduport");
    std::fs::create_dir_all(&ed_dir)?;
    let index_path = ed_dir.join("index.sqlite");
    let mut index = Index::open(&index_path)?;
    // Reconcile from disk so the index reflects on-disk state at
    // startup. The schema is loaded eagerly so custom-property
    // indexing works on the first reconcile.
    let schema = schema_store.current().ok();
    index::reconcile(index.conn_mut(), &entity_store, schema.as_ref())?;

    let state = Arc::new(EduportState {
        data_folder: data_folder.clone(),
        user_email: settings.user_email.clone(),
        entity_store,
        schema_store,
        view_store,
        index: Mutex::new(index),
        watcher: Mutex::new(None),
    });

    start_watcher(&state, app_handle.clone())?;

    Ok(state)
}

/// Spin up the file watcher, wiring its callback to forward typed
/// `VaultEvent`s as Tauri events on the `main` window. Each event
/// carries a JSON payload the frontend can deserialise without
/// looking at the Rust types.
fn start_watcher(state: &Arc<EduportState>, app_handle: tauri::AppHandle) -> Result<(), BootError> {
    let state_for_callback = Arc::clone(state);
    let watcher = Watcher::start(&state.data_folder, DEFAULT_DEBOUNCE, move |event| {
        // The watcher's worker thread is hot — keep this cheap.
        // We do *just enough* to keep the index in sync, then
        // emit a Tauri event so the frontend can refresh.
        handle_watcher_event(&state_for_callback, &app_handle, event);
    })?;
    *state
        .watcher
        .lock()
        .map_err(|_| eduport_core::EduportError::Poisoned("watcher"))? = Some(watcher);
    Ok(())
}

fn handle_watcher_event(
    state: &Arc<EduportState>,
    app_handle: &tauri::AppHandle,
    event: VaultEvent,
) {
    use eduport_core::index::writer::{
        clear_parse_error, delete_entity as index_delete, record_parse_error, upsert_entity,
    };

    match &event {
        VaultEvent::EntityChanged { path, file_id } => {
            // Try to read+parse the file and upsert. On failure,
            // record a parse error and let the frontend surface it.
            // Type is derived from the file's frontmatter tag — the
            // watcher itself is type-agnostic.
            let parse_result = read_and_parse(path);
            let mtime_ns = path
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_nanos() as i64)
                .unwrap_or(0);
            let schema = state.schema_store.current().ok();
            let index = state.index.lock().expect("index mutex poisoned");
            match parse_result {
                Ok((entity, body)) => {
                    let _ = upsert_entity(
                        index.conn(),
                        file_id,
                        path,
                        mtime_ns,
                        &entity,
                        &body,
                        schema.as_ref(),
                    );
                    let _ = clear_parse_error(index.conn(), &path.to_string_lossy());
                }
                Err(message) => {
                    let _ = record_parse_error(index.conn(), &path.to_string_lossy(), &message);
                    drop(index);
                    let _ = app_handle.emit(
                        "eduport:parse-error",
                        serde_json::json!({
                            "path": path.to_string_lossy(),
                            "message": message,
                        }),
                    );
                    return;
                }
            }
        }
        VaultEvent::EntityDeleted { file_id, .. } => {
            let index = state.index.lock().expect("index mutex poisoned");
            let _ = index_delete(index.conn(), file_id);
        }
        VaultEvent::SchemaChanged => {
            // Pull the new schema into the in-process cache so the
            // FTS5 `custom_text` column (which uses the schema to
            // decide which fields contribute searchable prose) is
            // rebuilt the next time an entity is written. The
            // historical "rebuild the SQL properties index" step is
            // gone — filtering is now `Vault::query`-based, which
            // reads the on-disk frontmatter directly.
            let _ = state.schema_store.reload();
        }
        VaultEvent::ViewsChanged => {
            let _ = state.view_store.reload();
        }
        VaultEvent::NeedsRescan => {
            let mut index = state.index.lock().expect("index mutex poisoned");
            let schema = state.schema_store.current().ok();
            let _ = eduport_core::index::reconcile(
                index.conn_mut(),
                &state.entity_store,
                schema.as_ref(),
            );
        }
    }

    // Forward a typed event the frontend can react to.
    let _ = app_handle.emit("eduport:vault-event", event_payload(&event));
}

/// Translate a [`VaultEvent`] into the JSON payload the frontend
/// consumes. The shape mirrors what the Python sidecar emitted as
/// SSE events so the frontend swap (Phase 10) doesn't need a
/// payload-shape change at the same time as the transport swap.
fn event_payload(event: &VaultEvent) -> serde_json::Value {
    use serde_json::json;
    match event {
        // Type is no longer carried by the watcher event — the
        // file's `eduport-type/<value>` tag is the source of truth
        // and the consumer (or the index) looks it up.
        VaultEvent::EntityChanged { file_id, path } => json!({
            "kind": "entity_changed",
            "file_id": file_id,
            "path": path.to_string_lossy(),
        }),
        VaultEvent::EntityDeleted { file_id, path } => json!({
            "kind": "entity_deleted",
            "file_id": file_id,
            "path": path.to_string_lossy(),
        }),
        VaultEvent::SchemaChanged => json!({"kind": "schema_changed"}),
        VaultEvent::ViewsChanged => json!({"kind": "views_changed"}),
        VaultEvent::NeedsRescan => json!({"kind": "needs_rescan"}),
    }
}

fn read_and_parse(path: &Path) -> Result<(eduport_core::entity::Entity, String), String> {
    let record = vaultdb_core::frontmatter::load_record_with_content(path)
        .map_err(|e| format!("read failed: {e}"))?;
    let raw = record.raw_content.as_deref().unwrap_or("");
    let body = match vaultdb_core::frontmatter::extract_frontmatter(raw) {
        Some((_, body_start)) => raw[body_start..].to_string(),
        None => return Err("missing or malformed `---` frontmatter delimiters".into()),
    };
    // The vault_root used for virtual fields (_path, _modified, etc.)
    // is the path's parent — best-effort, since the watcher doesn't
    // carry the vault handle here. Virtual fields are only consumed
    // by typed accessors; the entity body itself doesn't depend on
    // them.
    let vault_root = path.parent().unwrap_or(path);
    let entity = eduport_core::entity::Entity::from_record(&record, vault_root)?;
    Ok((entity, body))
}

// ── Helpers used by command handlers ──────────────────────────────

/// Stop the watcher and drop the in-process state. Called on app
/// shutdown and when the user changes their data folder.
pub fn shutdown(handle: &EduportStateHandle) {
    let mut guard = handle.lock().expect("eduport state handle poisoned");
    if let Some(state) = guard.take() {
        let _watcher = state.watcher.lock().expect("watcher mutex poisoned").take();
        // Dropping `state` (Arc) and `_watcher` (Watcher) stops the
        // notify threads; the Connection drops with the Arc.
    }
}
