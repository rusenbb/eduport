//! Walk the vault, compare on-disk mtimes with the indexed mtimes,
//! and bring the two into agreement.
//!
//! Reconcile is the cold-start path (and the recovery path after
//! anything that bypasses the watcher — sync programs writing
//! straight to the vault, manual `cp` operations, restoring from
//! backup). The watcher (Phase 8) handles the steady-state
//! incremental updates.
//!
//! ## Layout
//!
//! Reconcile walks the vault root **non-recursively**. Every
//! entity file sits at the root; subfolders are user-managed
//! Obsidian content (general notes, attachments) that this layer
//! intentionally ignores even when they contain `.md` files.
//! Type discrimination comes from each file's
//! `eduport-type/<value>` tag — *not* its location.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use rusqlite::Connection;

use crate::entity::{Entity, EntityStore};
use crate::schema::Schema;

use super::IndexError;
use super::writer::{clear_parse_error, delete_entity, record_parse_error, upsert_entity};

/// Outcome of one [`reconcile`] pass. Numbers add up to "files touched
/// during the walk"; `unchanged` is the cheap-path count.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReconcileSummary {
    /// New files indexed for the first time.
    pub added: usize,
    /// Existing files whose mtime changed and were re-indexed.
    pub updated: usize,
    /// File rows that no longer exist on disk and were removed.
    pub removed: usize,
    /// Files that matched the indexed mtime — cheapest path.
    pub unchanged: usize,
    /// Files whose frontmatter wouldn't parse; their `parse_errors`
    /// row is updated and the index entry left untouched.
    pub errors: usize,
}

/// Bring the index up to date with the on-disk vault state.
///
/// Walks the vault root non-recursively, stat-checks each `.md`
/// file, and:
///
/// - Skips files whose `mtime_ns` matches the cached value (fast path).
/// - Re-parses changed files and re-upserts them.
/// - Records a parse error and skips the upsert when frontmatter
///   doesn't parse — the row keeps its previous content so we don't
///   lose searchability for one bad edit.
/// - Deletes index rows whose file is no longer on disk.
///
/// `schema` is optional: when supplied, the upsert path also rebuilds
/// the `properties` table and the FTS5 `custom_text` column. Without
/// a schema the index still works for body/name/tags search.
pub fn reconcile(
    conn: &Connection,
    store: &EntityStore,
    schema: Option<&Schema>,
) -> Result<ReconcileSummary, IndexError> {
    let mut summary = ReconcileSummary::default();

    // Snapshot the indexed (file_id, mtime) pairs before the walk so
    // we can detect deletions in O(1) per file. Memory cost is one
    // i64 + a small string per entity; trivial at vault sizes we
    // target.
    let existing: HashMap<String, i64> = {
        let mut stmt = conn.prepare("SELECT file_id, mtime_ns FROM entities")?;
        stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?
            .collect::<rusqlite::Result<HashMap<_, _>>>()?
    };

    let mut seen: HashSet<String> = HashSet::new();
    walk_root(conn, store, schema, &existing, &mut seen, &mut summary)?;

    // Anything in `existing` that we didn't see on disk is gone.
    for file_id in existing.keys() {
        if !seen.contains(file_id) {
            delete_entity(conn, file_id)?;
            summary.removed += 1;
        }
    }

    Ok(summary)
}

/// Walk the vault root non-recursively, updating the index for
/// every `.md` file whose mtime has moved. Files in subdirectories
/// are intentionally skipped — see the module-level doc.
fn walk_root(
    conn: &Connection,
    store: &EntityStore,
    schema: Option<&Schema>,
    existing: &HashMap<String, i64>,
    seen: &mut HashSet<String>,
    summary: &mut ReconcileSummary,
) -> Result<(), IndexError> {
    let root = &store.vault().root;
    let entries = match std::fs::read_dir(root) {
        Ok(it) => it,
        Err(_) => return Ok(()),
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();

        // Skip subdirectories outright. Symlinks-to-files are
        // followed because they could be a sync tool's
        // implementation detail; subdir traversal is what we want
        // to avoid.
        if path.is_dir() {
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if stem.starts_with('.') {
            continue;
        }

        let mtime_ns = match entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        {
            Some(d) => d.as_nanos() as i64,
            None => continue,
        };

        let file_id = stem.to_string();
        seen.insert(file_id.clone());

        if existing.get(&file_id) == Some(&mtime_ns) {
            summary.unchanged += 1;
            continue;
        }

        match load_entity_at(&path) {
            LoadResult::Ok { entity, body } => {
                upsert_entity(conn, &file_id, &path, mtime_ns, &entity, &body, schema)?;
                clear_parse_error(conn, &path.to_string_lossy())?;
                if existing.contains_key(&file_id) {
                    summary.updated += 1;
                } else {
                    summary.added += 1;
                }
            }
            LoadResult::ParseError(message) => {
                record_parse_error(conn, &path.to_string_lossy(), &message)?;
                summary.errors += 1;
            }
        }
    }
    Ok(())
}

/// Result of loading a single file off disk during reconcile.
/// `Ok` boxes its `Entity` so the enum size stays at one pointer
/// + discriminant regardless of the variant's payload.
enum LoadResult {
    Ok { entity: Box<Entity>, body: String },
    ParseError(String),
}

/// Read one file off disk and split it into (frontmatter→Entity, body).
/// Type is taken from the file's `eduport-type/<value>` tag inside
/// `Entity::from_yaml`; we don't need to know the type up front.
fn load_entity_at(path: &Path) -> LoadResult {
    let raw = match std::fs::read_to_string(path) {
        Ok(r) => r,
        Err(e) => return LoadResult::ParseError(format!("read failed: {e}")),
    };

    let (yaml, body) = match split_frontmatter(&raw) {
        Some(v) => v,
        None => {
            return LoadResult::ParseError(
                "missing or malformed `---` frontmatter delimiters".into(),
            );
        }
    };

    let entity = match Entity::from_yaml(yaml) {
        Ok(e) => e,
        Err(reason) => return LoadResult::ParseError(reason),
    };

    LoadResult::Ok {
        entity: Box::new(entity),
        body: body.to_string(),
    }
}

/// Strip a `---\nYAML\n---\n` frontmatter block from the head of `raw`
/// and return `(yaml_block, body)`. Returns `None` when the file
/// doesn't have a frontmatter prefix at all.
fn split_frontmatter(raw: &str) -> Option<(&str, &str)> {
    let trimmed = raw.strip_prefix("---\n")?;
    let close = trimmed.find("\n---\n")?;
    let yaml = &trimmed[..close];
    let body = &trimmed[close + "\n---\n".len()..];
    Some((yaml, body))
}

#[cfg(test)]
mod tests {
    use super::super::Index;
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_entity_at_root(root: &Path, stem: &str, name: &str, body: &str) {
        let path = root.join(format!("{stem}.md"));
        let yaml = format!("---\nname: {name}\ntags:\n  - eduport-type/note\n---\n{body}");
        fs::write(&path, yaml).unwrap();
    }

    fn setup_vault() -> (TempDir, EntityStore) {
        let tmp = TempDir::new().unwrap();
        let store = EntityStore::new(vaultdb_core::Vault::with_root(tmp.path().to_path_buf()));
        (tmp, store)
    }

    #[test]
    fn reconcile_picks_up_new_file() {
        let (tmp, store) = setup_vault();
        write_entity_at_root(tmp.path(), "hello", "Hello", "world");
        let index = Index::open_in_memory().unwrap();
        let summary = reconcile(index.conn(), &store, None).unwrap();
        assert_eq!(summary.added, 1);
        assert_eq!(summary.updated, 0);
        assert_eq!(summary.unchanged, 0);
        assert_eq!(summary.removed, 0);

        // A second pass with no on-disk changes should be all
        // unchanged.
        let summary = reconcile(index.conn(), &store, None).unwrap();
        assert_eq!(summary.unchanged, 1);
        assert_eq!(summary.added, 0);
    }

    #[test]
    fn reconcile_detects_modified_file() {
        let (tmp, store) = setup_vault();
        write_entity_at_root(tmp.path(), "a", "A", "v1");
        let index = Index::open_in_memory().unwrap();
        reconcile(index.conn(), &store, None).unwrap();

        // Force a deterministic "different mtime" by rolling the
        // cached value back to zero — sidesteps filesystem mtime
        // granularity (some FS round to the second).
        index
            .conn()
            .execute("UPDATE entities SET mtime_ns = 0 WHERE file_id = 'a'", [])
            .unwrap();
        write_entity_at_root(tmp.path(), "a", "A v2", "v2");

        let summary = reconcile(index.conn(), &store, None).unwrap();
        assert_eq!(summary.updated, 1, "stale mtime must trigger an update");
        let name: String = index
            .conn()
            .query_row("SELECT name FROM entities WHERE file_id = 'a'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(name, "A v2");
    }

    #[test]
    fn reconcile_removes_deleted_file() {
        let (tmp, store) = setup_vault();
        write_entity_at_root(tmp.path(), "gone", "Gone", "");
        let index = Index::open_in_memory().unwrap();
        reconcile(index.conn(), &store, None).unwrap();
        std::fs::remove_file(tmp.path().join("gone.md")).unwrap();
        let summary = reconcile(index.conn(), &store, None).unwrap();
        assert_eq!(summary.removed, 1);
        let count: i64 = index
            .conn()
            .query_row("SELECT COUNT(*) FROM entities", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn reconcile_records_parse_error_for_bad_frontmatter() {
        let (tmp, store) = setup_vault();
        std::fs::write(tmp.path().join("bad.md"), "no frontmatter here").unwrap();
        let index = Index::open_in_memory().unwrap();
        let summary = reconcile(index.conn(), &store, None).unwrap();
        assert_eq!(summary.errors, 1);
        let count: i64 = index
            .conn()
            .query_row("SELECT COUNT(*) FROM parse_errors", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn reconcile_skips_dot_files_and_non_md() {
        let (tmp, store) = setup_vault();
        std::fs::write(tmp.path().join(".hidden.md"), "anything").unwrap();
        std::fs::write(tmp.path().join("readme.txt"), "anything").unwrap();
        let index = Index::open_in_memory().unwrap();
        let summary = reconcile(index.conn(), &store, None).unwrap();
        assert_eq!(summary.added, 0);
        assert_eq!(summary.errors, 0);
    }

    #[test]
    fn reconcile_ignores_subfolder_md_files() {
        // The user's vault often has scratch markdown in
        // subfolders (general notes, attachments). Reconcile must
        // not pick those up — they're not entities.
        let (tmp, store) = setup_vault();
        fs::create_dir(tmp.path().join("notes")).unwrap();
        std::fs::write(
            tmp.path().join("notes/scratch.md"),
            "---\nname: Scratch\ntags:\n  - eduport-type/note\n---\n",
        )
        .unwrap();
        let index = Index::open_in_memory().unwrap();
        let summary = reconcile(index.conn(), &store, None).unwrap();
        assert_eq!(summary.added, 0);
        let count: i64 = index
            .conn()
            .query_row("SELECT COUNT(*) FROM entities", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }
}
