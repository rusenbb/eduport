//! `notify`-based file watcher that emits typed [`VaultEvent`]s.
//!
//! ## Why a separate module
//!
//! The Phase 7 index (`crate::index`) is a passive cache. Something
//! has to detect on-disk changes and call into [`crate::index::writer`]
//! when files arrive, change, or vanish. Two consumer paths matter:
//!
//! - **Steady-state** — the user edits a markdown file in Obsidian
//!   while the eduport app is running. We need to re-index before
//!   the next render.
//! - **Sync storms** — Dropbox / iCloud / Syncthing on initial sync
//!   can deliver hundreds of file events per second. Without
//!   coalescing, this would thrash the FTS5 index.
//!
//! The watcher debounces via [`notify_debouncer_full`] (rename-aware
//! event coalescing inside a configurable window) and classifies
//! each path into one of the [`VaultEvent`] variants. The consumer
//! decides what to do with each event (re-parse, delete, reindex).
//!
//! ## Self-write filtering
//!
//! When eduport itself writes a file (via [`crate::EntityStore`]), the
//! OS still raises a notify event for it. Without filtering, every
//! `save()` would round-trip through the watcher and re-parse a file
//! we already have in memory. The watcher exposes
//! [`Watcher::note_self_write`] for the entity store / index to
//! announce "this path was just written by us; suppress the next
//! event for it within the self-write window".
//!
//! Self-write entries time out automatically after
//! [`SELF_WRITE_WINDOW`]; entries are checked lazily on each lookup
//! (no cleanup thread).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use notify::{EventKind, RecursiveMode};
use notify_debouncer_full::{
    DebounceEventResult, Debouncer, RecommendedCache, new_debouncer, notify::RecommendedWatcher,
};

use crate::EntityType;
use crate::entity::store::FolderMap;
use crate::schema::store::SCHEMA_FILENAME;
use crate::view::store::VIEWS_FILENAME;

/// `.eduport/` config directory name. Mirror of the constant in
/// [`crate::settings`] / [`crate::view`] — kept here to avoid a
/// circular import for what's effectively a path constant.
pub const EDUPORT_CONFIG_DIR: &str = ".eduport";

/// Default debounce window. Long enough to coalesce sync-storm bursts
/// (Dropbox/iCloud often emit 5–10 events per file in <50 ms);
/// short enough that interactive saves still feel snappy.
pub const DEFAULT_DEBOUNCE: Duration = Duration::from_millis(200);

/// How long a self-write entry suppresses watcher events for its
/// path. Five seconds is generously long: it covers writes on
/// network filesystems where the OS event can lag the actual write
/// by a second or two.
pub const SELF_WRITE_WINDOW: Duration = Duration::from_secs(5);

/// Typed event the watcher hands to its callback.
///
/// The watcher does no parsing — it only classifies paths into one
/// of these variants. The consumer (eduport-tauri / index writer)
/// is responsible for parsing the file via [`crate::EntityStore`]
/// and updating the index.
#[derive(Debug, Clone, PartialEq)]
pub enum VaultEvent {
    /// An entity `.md` file under one of the entity-type folders was
    /// created or modified. Indexers should re-parse and upsert.
    EntityChanged {
        /// Which entity-type folder the file lives in.
        kind: EntityType,
        /// Absolute path to the file as observed.
        path: PathBuf,
        /// Filename stem (the canonical file_id everywhere in
        /// eduport), pre-extracted for callers that don't want to
        /// re-derive it.
        file_id: String,
    },
    /// An entity `.md` file was deleted. Indexers should remove its
    /// row by `file_id`.
    EntityDeleted {
        kind: EntityType,
        path: PathBuf,
        file_id: String,
    },
    /// `<vault>/.eduport/schema.yaml` was created or modified. The
    /// consumer should reload the schema and trigger a property
    /// re-index via [`crate::index::writer::reindex_all_properties`].
    SchemaChanged,
    /// `<vault>/.eduport/views.yaml` was created or modified. The
    /// consumer should reload the views file.
    ViewsChanged,
    /// notify reported a rescan / overflow event. The consumer
    /// should trigger a full [`crate::index::reconcile::reconcile`]
    /// because individual events were lost.
    NeedsRescan,
}

/// Live watcher handle. Drop it to stop watching and release the
/// underlying threads.
pub struct Watcher {
    /// notify-debouncer-full owns the OS-level watcher + a worker
    /// thread that emits `DebounceEventResult` after the configured
    /// timeout. Holding it in the struct keeps both alive; dropping
    /// the struct stops the threads.
    _debouncer: Debouncer<RecommendedWatcher, RecommendedCache>,

    /// Self-write registry. Shared with the worker closure that
    /// dispatches events; the worker checks this before forwarding
    /// to the user callback.
    self_writes: Arc<Mutex<HashMap<PathBuf, Instant>>>,
}

/// Watcher errors. Wraps notify and IO errors so the caller has a
/// single error type to thread through.
#[derive(Debug, thiserror::Error)]
pub enum WatcherError {
    #[error(transparent)]
    Notify(#[from] notify::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

impl Watcher {
    /// Start watching every entity-type folder under `vault_root` and
    /// the `.eduport/` config folder. Calls `on_event` for each
    /// classified event after debouncing.
    ///
    /// `on_event` runs on a background worker thread — keep it cheap.
    /// If you need to do heavy work, push the event onto a channel
    /// and process it on your own thread.
    pub fn start<F>(
        vault_root: &Path,
        folder_map: &FolderMap,
        debounce: Duration,
        on_event: F,
    ) -> Result<Self, WatcherError>
    where
        F: Fn(VaultEvent) + Send + Sync + 'static,
    {
        // Build a reverse map from absolute folder path → EntityType
        // so the worker can classify a notify event in O(1).
        let mut folder_to_kind: HashMap<PathBuf, EntityType> = HashMap::new();
        for kind in EntityType::ALL {
            let folder = vault_root.join(folder_map.folder_for(kind));
            folder_to_kind.insert(folder, kind);
        }
        let config_dir = vault_root.join(EDUPORT_CONFIG_DIR);
        let folder_to_kind = Arc::new(folder_to_kind);
        let config_dir_owned = config_dir.clone();

        let self_writes: Arc<Mutex<HashMap<PathBuf, Instant>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let self_writes_for_worker = Arc::clone(&self_writes);

        let on_event = Arc::new(on_event);
        let folder_to_kind_for_worker = Arc::clone(&folder_to_kind);

        let mut debouncer = new_debouncer(
            debounce,
            None,
            move |result: DebounceEventResult| {
                let events = match result {
                    Ok(evs) => evs,
                    Err(_errors) => {
                        // Lost events — ask the consumer for a full
                        // rescan rather than silently going stale.
                        on_event(VaultEvent::NeedsRescan);
                        return;
                    }
                };

                for ev in events {
                    if matches!(ev.event.kind, EventKind::Other) {
                        // notify::EventKind::Other is the rescan
                        // signal on backends that batch (Linux's
                        // inotify queue overflow, macOS fsevent
                        // coalescing).
                        on_event(VaultEvent::NeedsRescan);
                        continue;
                    }
                    for path in &ev.event.paths {
                        if let Some(vault_event) = classify(
                            path,
                            ev.event.kind,
                            &folder_to_kind_for_worker,
                            &config_dir_owned,
                        ) {
                            // Self-write filter — drop the event if
                            // we wrote this path ourselves recently.
                            let mut w = self_writes_for_worker.lock().unwrap();
                            sweep_expired(&mut w);
                            if w.contains_key(path) {
                                continue;
                            }
                            drop(w);
                            on_event(vault_event);
                        }
                    }
                }
            },
        )?;

        // Watch every entity-type folder. Each one's a sibling at the
        // vault root; we don't recurse below the entity folder
        // because eduport entity files live one level deep, never
        // nested.
        for kind in EntityType::ALL {
            let folder = vault_root.join(folder_map.folder_for(kind));
            // Folder may not exist yet (first-launch on an empty
            // vault) — create it so the watcher attaches.
            std::fs::create_dir_all(&folder)?;
            debouncer.watch(&folder, RecursiveMode::NonRecursive)?;
        }

        // Watch the config folder. Same belt-and-suspenders create-if-
        // missing — SchemaStore creates it on first save, but we
        // can't depend on save-order here.
        std::fs::create_dir_all(&config_dir)?;
        debouncer.watch(&config_dir, RecursiveMode::NonRecursive)?;

        Ok(Watcher {
            _debouncer: debouncer,
            self_writes,
        })
    }

    /// Mark `path` as having been written by eduport itself. The
    /// next watcher event for this path within [`SELF_WRITE_WINDOW`]
    /// will be suppressed instead of forwarded to the callback.
    ///
    /// Call this *before* the write hits the disk (or as part of the
    /// same sync block that performs the write) so the suppression
    /// race is on our side: the event has to come after the entry
    /// goes in.
    pub fn note_self_write(&self, path: &Path) {
        let mut writes = self.self_writes.lock().unwrap();
        writes.insert(path.to_path_buf(), Instant::now());
    }
}

/// Drop expired self-write entries. Called inline on each lookup so
/// we don't need a cleanup thread; cost is amortised across event
/// dispatches and stays O(N) in the active-write count (which is at
/// most a handful at any instant).
fn sweep_expired(map: &mut HashMap<PathBuf, Instant>) {
    let now = Instant::now();
    map.retain(|_, t| now.duration_since(*t) < SELF_WRITE_WINDOW);
}

/// Classify a single (path, EventKind) pair into a [`VaultEvent`].
/// Returns `None` for paths we don't care about (non-`.md` under an
/// entity folder, files under `.eduport/` other than schema/views,
/// hidden files, etc.).
fn classify(
    path: &Path,
    kind: EventKind,
    folder_to_kind: &HashMap<PathBuf, EntityType>,
    config_dir: &Path,
) -> Option<VaultEvent> {
    // Hidden files and non-markdown files in entity folders are not
    // entities. The Python sidecar applied the same filter.
    let file_name = path.file_name()?.to_str()?;
    if file_name.starts_with('.') {
        return None;
    }

    let parent = path.parent()?;
    if parent == config_dir {
        if !is_create_or_modify(kind) && !is_remove(kind) {
            return None;
        }
        return match file_name {
            n if n == SCHEMA_FILENAME => Some(VaultEvent::SchemaChanged),
            n if n == VIEWS_FILENAME => Some(VaultEvent::ViewsChanged),
            _ => None,
        };
    }

    if path.extension().and_then(|s| s.to_str()) != Some("md") {
        return None;
    }

    let entity_kind = folder_to_kind.get(parent).copied()?;
    let stem = path.file_stem()?.to_str()?.to_string();

    if is_remove(kind) {
        Some(VaultEvent::EntityDeleted {
            kind: entity_kind,
            path: path.to_path_buf(),
            file_id: stem,
        })
    } else if is_create_or_modify(kind) {
        Some(VaultEvent::EntityChanged {
            kind: entity_kind,
            path: path.to_path_buf(),
            file_id: stem,
        })
    } else {
        None
    }
}

fn is_create_or_modify(kind: EventKind) -> bool {
    matches!(kind, EventKind::Create(_) | EventKind::Modify(_))
}

fn is_remove(kind: EventKind) -> bool {
    matches!(kind, EventKind::Remove(_))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::store::FolderMap;
    use std::fs;
    use std::sync::mpsc;
    use tempfile::TempDir;

    fn setup_vault() -> (TempDir, FolderMap) {
        let tmp = TempDir::new().unwrap();
        let folder_map = FolderMap::default();
        for kind in EntityType::ALL {
            std::fs::create_dir_all(tmp.path().join(folder_map.folder_for(kind))).unwrap();
        }
        std::fs::create_dir_all(tmp.path().join(EDUPORT_CONFIG_DIR)).unwrap();
        (tmp, folder_map)
    }

    /// Wait for an event up to `dur`. Returns None on timeout.
    fn recv_within(rx: &mpsc::Receiver<VaultEvent>, dur: Duration) -> Option<VaultEvent> {
        rx.recv_timeout(dur).ok()
    }

    #[test]
    fn classify_entity_create_under_notes_folder() {
        let (tmp, folder_map) = setup_vault();
        let mut map: HashMap<PathBuf, EntityType> = HashMap::new();
        for kind in EntityType::ALL {
            map.insert(tmp.path().join(folder_map.folder_for(kind)), kind);
        }
        let path = tmp.path().join(folder_map.folder_for(EntityType::Note)).join("hello.md");
        let ev = classify(
            &path,
            EventKind::Create(notify::event::CreateKind::File),
            &map,
            &tmp.path().join(EDUPORT_CONFIG_DIR),
        )
        .unwrap();
        match ev {
            VaultEvent::EntityChanged { kind, file_id, .. } => {
                assert_eq!(kind, EntityType::Note);
                assert_eq!(file_id, "hello");
            }
            _ => panic!("expected EntityChanged"),
        }
    }

    #[test]
    fn classify_skips_non_md_under_entity_folder() {
        let (tmp, folder_map) = setup_vault();
        let mut map: HashMap<PathBuf, EntityType> = HashMap::new();
        map.insert(tmp.path().join(folder_map.folder_for(EntityType::Note)), EntityType::Note);
        let path = tmp.path().join(folder_map.folder_for(EntityType::Note)).join("README.txt");
        assert!(classify(
            &path,
            EventKind::Create(notify::event::CreateKind::File),
            &map,
            &tmp.path().join(EDUPORT_CONFIG_DIR),
        )
        .is_none());
    }

    #[test]
    fn classify_skips_hidden_files() {
        let (tmp, folder_map) = setup_vault();
        let mut map: HashMap<PathBuf, EntityType> = HashMap::new();
        map.insert(tmp.path().join(folder_map.folder_for(EntityType::Note)), EntityType::Note);
        let path = tmp.path().join(folder_map.folder_for(EntityType::Note)).join(".swp");
        assert!(classify(
            &path,
            EventKind::Create(notify::event::CreateKind::File),
            &map,
            &tmp.path().join(EDUPORT_CONFIG_DIR),
        )
        .is_none());
    }

    #[test]
    fn classify_schema_yaml_emits_schema_changed() {
        let (tmp, _) = setup_vault();
        let map: HashMap<PathBuf, EntityType> = HashMap::new();
        let config = tmp.path().join(EDUPORT_CONFIG_DIR);
        let path = config.join(SCHEMA_FILENAME);
        let ev = classify(
            &path,
            EventKind::Modify(notify::event::ModifyKind::Data(notify::event::DataChange::Any)),
            &map,
            &config,
        )
        .unwrap();
        assert_eq!(ev, VaultEvent::SchemaChanged);
    }

    #[test]
    fn classify_views_yaml_emits_views_changed() {
        let (tmp, _) = setup_vault();
        let map: HashMap<PathBuf, EntityType> = HashMap::new();
        let config = tmp.path().join(EDUPORT_CONFIG_DIR);
        let path = config.join(VIEWS_FILENAME);
        let ev = classify(
            &path,
            EventKind::Create(notify::event::CreateKind::File),
            &map,
            &config,
        )
        .unwrap();
        assert_eq!(ev, VaultEvent::ViewsChanged);
    }

    #[test]
    fn classify_remove_emits_entity_deleted() {
        let (tmp, folder_map) = setup_vault();
        let mut map: HashMap<PathBuf, EntityType> = HashMap::new();
        map.insert(tmp.path().join(folder_map.folder_for(EntityType::Note)), EntityType::Note);
        let path = tmp.path().join(folder_map.folder_for(EntityType::Note)).join("gone.md");
        let ev = classify(
            &path,
            EventKind::Remove(notify::event::RemoveKind::File),
            &map,
            &tmp.path().join(EDUPORT_CONFIG_DIR),
        )
        .unwrap();
        match ev {
            VaultEvent::EntityDeleted { file_id, .. } => assert_eq!(file_id, "gone"),
            _ => panic!("expected EntityDeleted"),
        }
    }

    #[test]
    fn sweep_expired_drops_old_entries() {
        let mut map: HashMap<PathBuf, Instant> = HashMap::new();
        map.insert(
            PathBuf::from("/old"),
            Instant::now() - SELF_WRITE_WINDOW * 2,
        );
        map.insert(PathBuf::from("/fresh"), Instant::now());
        sweep_expired(&mut map);
        assert!(!map.contains_key(Path::new("/old")));
        assert!(map.contains_key(Path::new("/fresh")));
    }

    // ── Live integration tests ──────────────────────────────────
    //
    // These spin up a real notify watcher against a tempdir. They're
    // inherently timing-sensitive — debouncer-full uses a 1/4-of-
    // timeout tick rate by default. We use a short debounce
    // (50 ms) and a generous wait (2 s) to keep the tests reliable
    // on slow CI.

    #[test]
    fn live_watcher_emits_entity_changed_on_create() {
        let (tmp, folder_map) = setup_vault();
        let (tx, rx) = mpsc::channel();
        let _watcher = Watcher::start(
            tmp.path(),
            &folder_map,
            Duration::from_millis(50),
            move |ev| {
                let _ = tx.send(ev);
            },
        )
        .expect("start watcher");

        let path = tmp.path().join(folder_map.folder_for(EntityType::Note)).join("hi.md");
        fs::write(&path, "---\nname: Hi\ntags:\n  - eduport-type/note\n---\n").unwrap();

        let ev = recv_within(&rx, Duration::from_secs(2))
            .expect("watcher should emit an event for the new file");
        match ev {
            VaultEvent::EntityChanged { kind, file_id, .. } => {
                assert_eq!(kind, EntityType::Note);
                assert_eq!(file_id, "hi");
            }
            other => panic!("unexpected event: {:?}", other),
        }
    }

    #[test]
    fn live_watcher_self_write_filter_suppresses_event() {
        let (tmp, folder_map) = setup_vault();
        let (tx, rx) = mpsc::channel();
        let watcher = Watcher::start(
            tmp.path(),
            &folder_map,
            Duration::from_millis(50),
            move |ev| {
                let _ = tx.send(ev);
            },
        )
        .expect("start watcher");

        let path = tmp.path().join(folder_map.folder_for(EntityType::Note)).join("self.md");
        watcher.note_self_write(&path);
        fs::write(&path, "---\nname: Self\ntags:\n  - eduport-type/note\n---\n").unwrap();

        // Should not receive any event within a reasonable window.
        assert!(recv_within(&rx, Duration::from_millis(500)).is_none(),
            "self-write should suppress the watcher event");
    }

    #[test]
    fn live_watcher_emits_schema_changed() {
        let (tmp, folder_map) = setup_vault();
        let (tx, rx) = mpsc::channel();
        let _watcher = Watcher::start(
            tmp.path(),
            &folder_map,
            Duration::from_millis(50),
            move |ev| {
                let _ = tx.send(ev);
            },
        )
        .expect("start watcher");

        let path = tmp.path().join(EDUPORT_CONFIG_DIR).join(SCHEMA_FILENAME);
        fs::write(&path, "version: 1\ntypes:\n").unwrap();

        let mut saw_schema = false;
        // The OS may emit several events for one write; consume until
        // we see the schema event or time out.
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if let Some(ev) = recv_within(&rx, Duration::from_millis(200))
                && matches!(ev, VaultEvent::SchemaChanged)
            {
                saw_schema = true;
                break;
            }
        }
        assert!(saw_schema, "expected at least one SchemaChanged event");
    }
}
