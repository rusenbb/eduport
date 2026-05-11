//! Mutators for the FTS5 index.
//!
//! Free functions that take `&rusqlite::Connection`, so the watcher
//! (Phase 8) can wrap a batch of upserts in a single transaction. See
//! [`super`] for the rationale.
//!
//! Each top-level mutator (`upsert_entity`, `delete_entity`, etc.)
//! commits its own transaction when called against a bare connection
//! (`is_autocommit() == true`). When called against a connection that
//! already has an open transaction, the outer transaction wins — the
//! mutator becomes a participant rather than a transaction boundary.

use std::path::Path;

use rusqlite::{Connection, params};

use crate::entity::Entity;
use crate::schema::{PropertyKind, Schema};

use super::IndexError;

/// Update or insert the index row for a single entity.
///
/// `file_id` is the filename stem (the convention everywhere in
/// eduport — `students/jane-smith-a3kf.md` → `jane-smith-a3kf`).
/// `mtime_ns` lets the reconcile layer skip up-to-date files.
/// `body` is the markdown body after the closing `---`.
/// `schema` is optional — when present, it drives the `properties`
/// table and the FTS5 `custom_text` column so custom-field search
/// and filter work; when absent, only the canonical entity row,
/// tags, and a body+name+tags-only FTS5 row are written.
pub fn upsert_entity(
    conn: &Connection,
    file_id: &str,
    path: &Path,
    mtime_ns: i64,
    entity: &Entity,
    body: &str,
    schema: Option<&Schema>,
) -> Result<(), IndexError> {
    // Serialise the frontmatter to YAML so we can rehydrate without
    // re-reading the file (the schema-rewrite reindex path uses this).
    // YAML is a superset of JSON for our purposes; `Entity::from_yaml`
    // round-trips it.
    let fm_yaml = entity
        .to_yaml()
        .map_err(|e| IndexError::Data(format!("frontmatter serialise: {}", e)))?;

    with_optional_tx(conn, |conn| {
        conn.execute(
            "INSERT OR REPLACE INTO entities \
             (file_id, type, name, path, mtime_ns, body, frontmatter) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                file_id,
                entity.entity_type().as_str(),
                entity.name(),
                path.to_string_lossy().as_ref(),
                mtime_ns,
                body,
                fm_yaml,
            ],
        )?;

        let rowid: i64 = conn.query_row(
            "SELECT rowid FROM entities WHERE file_id = ?1",
            params![file_id],
            |r| r.get(0),
        )?;

        // FTS5 row: explicit delete-then-insert. INSERT OR REPLACE
        // doesn't compose well with FTS5 virtual tables, and the
        // contentful-table semantics make the round-trip ambiguous.
        // The `entity_tags` and `properties` tables that used to be
        // written here are gone — filter evaluation now goes through
        // `Vault::query`, which reads tags + custom fields straight
        // from the on-disk frontmatter (no shadow index to keep in
        // sync).
        conn.execute("DELETE FROM entities_fts WHERE rowid = ?1", params![rowid])?;
        let custom_text = match schema {
            Some(s) => custom_text_for_fts5(entity, s),
            None => String::new(),
        };
        let tags_joined = entity.tags().join(" ");
        conn.execute(
            "INSERT INTO entities_fts(rowid, body, name, tags, custom_text) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![rowid, body, entity.name(), tags_joined, custom_text],
        )?;
        Ok(())
    })
}

/// Remove an entity and all its dependent rows from the index. The
/// FTS5 table doesn't cascade through SQLite foreign keys (it's a
/// virtual table), so we delete that row explicitly first; the
/// rest cascade via the FK declarations on `entities.file_id`.
pub fn delete_entity(conn: &Connection, file_id: &str) -> Result<(), IndexError> {
    with_optional_tx(conn, |conn| {
        let rowid: Option<i64> = conn
            .query_row(
                "SELECT rowid FROM entities WHERE file_id = ?1",
                params![file_id],
                |r| r.get(0),
            )
            .ok();
        if let Some(rowid) = rowid {
            conn.execute("DELETE FROM entities_fts WHERE rowid = ?1", params![rowid])?;
        }
        conn.execute("DELETE FROM entities WHERE file_id = ?1", params![file_id])?;
        Ok(())
    })
}

/// Record (or replace) a parse error for a path. Surfaced to the UI
/// via the watcher's `parse-error` event — see `src/watcher.rs`
/// (Phase 8).
pub fn record_parse_error(conn: &Connection, path: &str, message: &str) -> Result<(), IndexError> {
    conn.execute(
        "INSERT OR REPLACE INTO parse_errors(path, message) VALUES (?1, ?2)",
        params![path, message],
    )?;
    Ok(())
}

/// Clear a parse error after a successful re-parse.
pub fn clear_parse_error(conn: &Connection, path: &str) -> Result<(), IndexError> {
    conn.execute("DELETE FROM parse_errors WHERE path = ?1", params![path])?;
    Ok(())
}

/// Run `f` inside a transaction, but only if the connection is
/// currently in autocommit mode. When the caller has already opened
/// a transaction (the watcher batch path), `f` runs without nesting
/// — SQLite doesn't support nested BEGINs and we don't want to use
/// SAVEPOINTs because the outer caller is still the transaction
/// owner and decides whether to COMMIT or ROLLBACK.
fn with_optional_tx<F>(conn: &Connection, f: F) -> Result<(), IndexError>
where
    F: FnOnce(&Connection) -> Result<(), IndexError>,
{
    let owns_tx = conn.is_autocommit();
    if owns_tx {
        conn.execute("BEGIN IMMEDIATE", [])?;
    }
    let result = f(conn);
    if owns_tx {
        match &result {
            Ok(()) => {
                conn.execute("COMMIT", [])?;
            }
            Err(_) => {
                let _ = conn.execute("ROLLBACK", []);
            }
        }
    }
    result
}

/// Concatenate `text` / `url` custom-property values into the FTS5
/// `custom_text` column so command-palette search matches against
/// custom-field prose. Other property types (numbers, dates,
/// multi-select) don't carry searchable prose and are skipped.
fn custom_text_for_fts5(entity: &Entity, schema: &Schema) -> String {
    let extras = entity.custom();
    if extras.is_empty() {
        return String::new();
    }
    let type_schema = schema.for_type(entity.entity_type());
    let mut parts: Vec<String> = Vec::new();
    for (key, value) in extras {
        let Some(prop) = type_schema.property(key) else {
            continue;
        };
        if matches!(prop.kind(), PropertyKind::Text | PropertyKind::Url)
            && let serde_yaml::Value::String(s) = value
        {
            parts.push(s.clone());
        }
    }
    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::super::Index;
    use super::*;
    use crate::entity::{Entity, Note};
    use crate::entity_type::EntityType;
    use crate::schema::{Property, TextProperty, empty_schema};

    fn note(name: &str, tags: &[&str]) -> Entity {
        let mut n = Note {
            name: name.into(),
            tags: vec!["eduport-type/note".into()],
            custom: Default::default(),
        };
        for t in tags {
            n.tags.push((*t).into());
        }
        Entity::Note(n)
    }

    #[test]
    fn upsert_then_query_round_trip() {
        let index = Index::open_in_memory().expect("open");
        let entity = note("Hello", &["greeting"]);
        upsert_entity(
            index.conn(),
            "hello-1234",
            std::path::Path::new("notes/hello-1234.md"),
            42,
            &entity,
            "world body",
            None,
        )
        .expect("upsert");

        let (file_id, type_, name, mtime_ns): (String, String, String, i64) = index
            .conn()
            .query_row(
                "SELECT file_id, type, name, mtime_ns FROM entities",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert_eq!(file_id, "hello-1234");
        assert_eq!(type_, EntityType::Note.as_str());
        assert_eq!(name, "Hello");
        assert_eq!(mtime_ns, 42);
    }

    #[test]
    fn delete_cascades_to_fts() {
        let index = Index::open_in_memory().expect("open");
        let entity = note("Y", &["t1"]);
        upsert_entity(
            index.conn(),
            "y",
            std::path::Path::new("y.md"),
            0,
            &entity,
            "body",
            None,
        )
        .unwrap();
        delete_entity(index.conn(), "y").unwrap();

        let entity_count: i64 = index
            .conn()
            .query_row("SELECT COUNT(*) FROM entities", [], |r| r.get(0))
            .unwrap();
        let fts_count: i64 = index
            .conn()
            .query_row("SELECT COUNT(*) FROM entities_fts", [], |r| r.get(0))
            .unwrap();
        assert_eq!(entity_count, 0);
        assert_eq!(fts_count, 0);
    }

    #[test]
    fn parse_error_record_and_clear() {
        let index = Index::open_in_memory().expect("open");
        record_parse_error(index.conn(), "/tmp/bad.md", "bad frontmatter").unwrap();
        let n: i64 = index
            .conn()
            .query_row("SELECT COUNT(*) FROM parse_errors", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
        clear_parse_error(index.conn(), "/tmp/bad.md").unwrap();
        let n: i64 = index
            .conn()
            .query_row("SELECT COUNT(*) FROM parse_errors", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn upsert_with_schema_populates_fts_custom_text() {
        let index = Index::open_in_memory().expect("open");
        let mut schema = empty_schema();
        schema.types.get_mut(&EntityType::Note).unwrap().properties =
            vec![Property::Text(TextProperty {
                key: "summary".into(),
                name: "Summary".into(),
                description: None,
                required: false,
                default: None,
            })];

        let mut n = Note {
            name: "Howdy".into(),
            tags: vec!["eduport-type/note".into()],
            custom: Default::default(),
        };
        n.custom.insert(
            "summary".into(),
            serde_yaml::Value::String("hello there".into()),
        );

        let entity = Entity::Note(n);
        upsert_entity(
            index.conn(),
            "howdy",
            std::path::Path::new("howdy.md"),
            0,
            &entity,
            "",
            Some(&schema),
        )
        .unwrap();

        // The custom-property value should land in the FTS5
        // `custom_text` column so command-palette search can match
        // against it. The bespoke `properties` table is gone — only
        // the FTS-side text projection remains.
        let custom_text: String = index
            .conn()
            .query_row(
                "SELECT custom_text FROM entities_fts WHERE rowid = (SELECT rowid FROM entities WHERE file_id = 'howdy')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(custom_text, "hello there");
    }
}
