//! Reconcile path — delegates the walk + diff to
//! `vaultdb_fts::reconcile` with an eduport-shaped projection
//! closure.
//!
//! On top of vaultdb-fts's work we still:
//!   - Resolve the `Schema` (so `upsert_entity` can compute
//!     `custom_text` for the FTS5 column)
//!   - Sweep stale `parse_errors` rows (files no longer at the vault
//!     root, or files that have moved into subfolders)
//!   - Re-run our type-aware `Entity::from_yaml` on each record so we
//!     can record a parse error for the user if frontmatter doesn't
//!     match an eduport entity shape

use std::path::Path;

use rusqlite::Connection;

use crate::entity::{Entity, EntityStore};
use crate::schema::Schema;

use super::IndexError;
use super::writer::{clear_parse_error, record_parse_error};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReconcileSummary {
    pub added: usize,
    pub updated: usize,
    pub removed: usize,
    pub unchanged: usize,
    pub errors: usize,
}

pub fn reconcile(
    conn: &Connection,
    store: &EntityStore,
    schema: Option<&Schema>,
) -> Result<ReconcileSummary, IndexError> {
    let mut summary = ReconcileSummary::default();

    // Track per-file parse outcomes so we can sweep parse_errors for
    // files that parsed fine on this pass.
    let vault = store.vault();
    let read_dir = std::fs::read_dir(&vault.root)?;

    let mut paths: Vec<std::path::PathBuf> = Vec::new();
    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        if path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.starts_with('.'))
            .unwrap_or(true)
        {
            continue;
        }
        paths.push(path);
    }

    let mut errors: Vec<(String, String)> = Vec::new();

    let raw = vaultdb_fts::reconcile(conn, vault, |record| {
        let path = &record.path;
        let raw_content = record.raw_content.as_deref().unwrap_or("");
        let (yaml, body) = match split_frontmatter(raw_content) {
            Some(v) => v,
            None => {
                errors.push((
                    path.to_string_lossy().into_owned(),
                    "missing or malformed `---` frontmatter delimiters".into(),
                ));
                return None;
            }
        };
        let entity = match Entity::from_yaml(yaml) {
            Ok(e) => e,
            Err(reason) => {
                errors.push((path.to_string_lossy().into_owned(), reason));
                return None;
            }
        };
        let file_id = path.file_stem()?.to_str()?.to_string();
        let mtime_ns = path
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_nanos() as i64)
            .unwrap_or(0);
        let custom_text = match schema {
            Some(s) => super::writer::custom_text_for_fts5(&entity, s),
            None => String::new(),
        };
        Some(vaultdb_fts::OwnedDocument {
            file_id,
            path: path.clone(),
            mtime_ns,
            body: body.to_string(),
            name: entity.name().to_string(),
            tags: entity.tags().to_vec(),
            custom_text,
        })
    })?;

    summary.added = raw.added;
    summary.updated = raw.updated;
    summary.removed = raw.removed;
    summary.unchanged = raw.unchanged;
    summary.errors = errors.len();

    for (path, msg) in &errors {
        record_parse_error(conn, path, msg)?;
    }

    // Sweep stale parse_errors: drop entries whose path is no longer a
    // top-level vault-root `.md` file. Covers two cases:
    //   (a) old version of the reconciler walked subfolders;
    //   (b) a previously-bad file was deleted on disk.
    let root = &vault.root;
    let stale: Vec<String> = {
        let mut stmt = conn.prepare("SELECT path FROM parse_errors")?;
        stmt.query_map([], |r| r.get::<_, String>(0))?
            .filter_map(std::result::Result::ok)
            .filter(|p| {
                let path = Path::new(p);
                path.parent() != Some(root.as_path()) || !path.exists()
            })
            .collect()
    };
    for p in stale {
        clear_parse_error(conn, &p)?;
    }

    Ok(summary)
}

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

        let summary = reconcile(index.conn(), &store, None).unwrap();
        assert_eq!(summary.unchanged, 1);
    }

    #[test]
    fn reconcile_removes_deleted_file() {
        let (tmp, store) = setup_vault();
        write_entity_at_root(tmp.path(), "gone", "Gone", "");
        let index = Index::open_in_memory().unwrap();
        reconcile(index.conn(), &store, None).unwrap();
        fs::remove_file(tmp.path().join("gone.md")).unwrap();
        let summary = reconcile(index.conn(), &store, None).unwrap();
        assert_eq!(summary.removed, 1);
    }

    #[test]
    fn reconcile_records_parse_error_for_bad_frontmatter() {
        let (tmp, store) = setup_vault();
        fs::write(tmp.path().join("bad.md"), "no frontmatter here").unwrap();
        let index = Index::open_in_memory().unwrap();
        let summary = reconcile(index.conn(), &store, None).unwrap();
        assert_eq!(summary.errors, 1);
        let n: i64 = index
            .conn()
            .query_row("SELECT COUNT(*) FROM parse_errors", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
    }
}
