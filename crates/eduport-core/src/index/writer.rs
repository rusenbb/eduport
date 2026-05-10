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
use crate::schema::{Property, PropertyKind, Schema};

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

        // Tags: replace-all (the entity is the source of truth).
        conn.execute(
            "DELETE FROM entity_tags WHERE file_id = ?1",
            params![file_id],
        )?;
        {
            let mut tag_stmt =
                conn.prepare("INSERT INTO entity_tags(file_id, tag) VALUES (?1, ?2)")?;
            for tag in entity.tags() {
                tag_stmt.execute(params![file_id, tag])?;
            }
        }

        // FTS5 row: explicit delete-then-insert. INSERT OR REPLACE
        // doesn't compose well with FTS5 virtual tables, and the
        // contentful-table semantics make the round-trip ambiguous.
        conn.execute(
            "DELETE FROM entities_fts WHERE rowid = ?1",
            params![rowid],
        )?;
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

        // Custom-property index: only when we have a schema to declare
        // which keys are properties (vs. arbitrary tail YAML). When no
        // schema is supplied we still wipe stale rows so a "load with
        // schema → load without" sequence doesn't leave orphans.
        match schema {
            Some(s) => upsert_properties(conn, file_id, entity, s)?,
            None => {
                conn.execute(
                    "DELETE FROM properties WHERE file_id = ?1",
                    params![file_id],
                )?;
            }
        }
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
            conn.execute(
                "DELETE FROM entities_fts WHERE rowid = ?1",
                params![rowid],
            )?;
        }
        conn.execute("DELETE FROM entities WHERE file_id = ?1", params![file_id])?;
        Ok(())
    })
}

/// Record (or replace) a parse error for a path. Surfaced to the UI
/// via the watcher's `parse-error` event — see `src/watcher.rs`
/// (Phase 8).
pub fn record_parse_error(
    conn: &Connection,
    path: &str,
    message: &str,
) -> Result<(), IndexError> {
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

/// Re-derive the `properties` table for every entity using the given
/// schema. Called after a schema mutation (add/patch/delete property)
/// so the SQL filter/sort surface stays in sync. Returns the number
/// of entities re-indexed.
pub fn reindex_all_properties(
    conn: &Connection,
    schema: &Schema,
) -> Result<usize, IndexError> {
    // Pull (file_id, frontmatter) pairs out first so we don't hold a
    // statement open during the per-row mutation.
    let pairs: Vec<(String, String)> = {
        let mut stmt = conn.prepare("SELECT file_id, frontmatter FROM entities")?;
        let rows = stmt.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };

    let mut n = 0;
    with_optional_tx(conn, |conn| {
        for (file_id, fm_yaml) in &pairs {
            let entity = match Entity::from_yaml(fm_yaml) {
                Ok(e) => e,
                Err(_) => continue, // schema reindex skips parse-broken rows
            };
            upsert_properties(conn, file_id, &entity, schema)?;
            n += 1;
        }
        Ok(())
    })?;
    Ok(n)
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

/// Pick the right `properties` columns for a single value. Returns
/// `(text, num, date, multi)` — exactly one is `Some` for known shapes;
/// all-`None` is returned when the value fails to match the property
/// type. The all-`None` case is silently skipped on insert (the
/// value-warnings layer surfaces type mismatches to the user
/// separately, so no error here).
fn coerce_property_columns(
    prop: &Property,
    value: &serde_yaml::Value,
) -> (Option<String>, Option<f64>, Option<String>, Option<String>) {
    let mut text = None;
    let mut num = None;
    let mut iso = None;
    let mut multi = None;

    match prop {
        Property::Text(_) | Property::Url(_) | Property::SingleSelect(_) => {
            if let serde_yaml::Value::String(s) = value {
                text = Some(s.clone());
            }
        }
        Property::Relation(_) => {
            // Store the raw wikilink string; resolved targets come
            // from vaultdb-core's link graph, not from this index.
            if let serde_yaml::Value::String(s) = value {
                text = Some(s.clone());
            }
        }
        Property::Number(_) => {
            // YAML "true" parses to Bool — explicitly reject so we
            // don't accidentally index a checkbox value as a number.
            if let serde_yaml::Value::Number(n) = value
                && let Some(f) = n.as_f64()
            {
                num = Some(f);
            }
        }
        Property::Checkbox(_) => {
            if let serde_yaml::Value::Bool(b) = value {
                num = Some(if *b { 1.0 } else { 0.0 });
            }
        }
        Property::Date(_) => {
            if let serde_yaml::Value::String(s) = value
                && iso_date_shaped(s)
            {
                iso = Some(s.clone());
            }
        }
        Property::MultiSelect(_) => {
            if let serde_yaml::Value::Sequence(seq) = value {
                let strs: Option<Vec<String>> = seq
                    .iter()
                    .map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                if let Some(strs) = strs
                    && let Ok(json) = serde_json::to_string(&strs)
                {
                    multi = Some(json);
                }
            }
        }
    }
    (text, num, iso, multi)
}

/// Lightweight ISO-8601 `YYYY-MM-DD` shape check. The full chrono
/// crate is overkill for "is this 10 chars shaped like a date"; the
/// reader uses lexicographic SQL compare on the stored strings.
fn iso_date_shaped(s: &str) -> bool {
    if s.len() != 10 {
        return false;
    }
    let bytes = s.as_bytes();
    bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[..4].iter().all(|b| b.is_ascii_digit())
        && bytes[5..7].iter().all(|b| b.is_ascii_digit())
        && bytes[8..10].iter().all(|b| b.is_ascii_digit())
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

/// Replace `properties` rows for one entity, skipping orphaned and
/// type-mismatched values. Public to the parent module so
/// [`reindex_all_properties`] and [`upsert_entity`] can share it.
pub(crate) fn upsert_properties(
    conn: &Connection,
    file_id: &str,
    entity: &Entity,
    schema: &Schema,
) -> Result<(), IndexError> {
    conn.execute(
        "DELETE FROM properties WHERE file_id = ?1",
        params![file_id],
    )?;
    let extras = entity.custom();
    if extras.is_empty() {
        return Ok(());
    }
    let type_schema = schema.for_type(entity.entity_type());

    let mut stmt = conn.prepare(
        "INSERT INTO properties \
         (file_id, key, type, value_text, value_num, value_date, value_multi) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )?;
    for (key, value) in extras {
        let Some(prop) = type_schema.property(key) else {
            continue;
        };
        let (text, num, iso, multi) = coerce_property_columns(prop, value);
        if text.is_none() && num.is_none() && iso.is_none() && multi.is_none() {
            continue;
        }
        stmt.execute(params![
            file_id,
            key.as_str(),
            prop.kind().as_str(),
            text,
            num,
            iso,
            multi,
        ])?;
    }
    Ok(())
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
    fn upsert_replaces_tags_atomically() {
        let index = Index::open_in_memory().expect("open");
        let mut entity = note("X", &["a", "b"]);
        upsert_entity(
            index.conn(),
            "x",
            std::path::Path::new("x.md"),
            0,
            &entity,
            "",
            None,
        )
        .unwrap();
        if let Entity::Note(n) = &mut entity {
            n.tags.clear();
            n.tags.push("eduport-type/note".into());
            n.tags.push("c".into());
        }
        upsert_entity(
            index.conn(),
            "x",
            std::path::Path::new("x.md"),
            1,
            &entity,
            "",
            None,
        )
        .unwrap();
        let mut stmt = index
            .conn()
            .prepare(
                "SELECT tag FROM entity_tags WHERE file_id = 'x' \
                 AND tag NOT LIKE 'eduport-type/%' ORDER BY tag",
            )
            .unwrap();
        let tags: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(tags, vec!["c"]);
    }

    #[test]
    fn delete_cascades_to_tags_and_fts() {
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
        let tag_count: i64 = index
            .conn()
            .query_row("SELECT COUNT(*) FROM entity_tags", [], |r| r.get(0))
            .unwrap();
        let fts_count: i64 = index
            .conn()
            .query_row("SELECT COUNT(*) FROM entities_fts", [], |r| r.get(0))
            .unwrap();
        assert_eq!(entity_count, 0);
        assert_eq!(tag_count, 0);
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
    fn iso_date_shape_validator() {
        assert!(iso_date_shaped("2025-01-30"));
        assert!(!iso_date_shaped("2025-1-30"));
        assert!(!iso_date_shaped("2025/01/30"));
        assert!(!iso_date_shaped("not-a-date"));
        assert!(!iso_date_shaped(""));
    }

    #[test]
    fn upsert_with_schema_indexes_text_property() {
        let index = Index::open_in_memory().expect("open");
        let mut schema = empty_schema();
        schema.types.get_mut(&EntityType::Note).unwrap().properties = vec![Property::Text(
            TextProperty {
                key: "summary".into(),
                name: "Summary".into(),
                description: None,
                required: false,
                default: None,
            },
        )];

        let mut n = Note {
            name: "Howdy".into(),
            tags: vec!["eduport-type/note".into()],
            custom: Default::default(),
        };
        n.custom
            .insert("summary".into(), serde_yaml::Value::String("hello there".into()));

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

        let (key, type_, val): (String, String, String) = index
            .conn()
            .query_row(
                "SELECT key, type, value_text FROM properties WHERE file_id = 'howdy'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(key, "summary");
        assert_eq!(type_, "text");
        assert_eq!(val, "hello there");

        // FTS5 should also have it
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

    #[test]
    fn schema_change_reindex_picks_up_renamed_keys() {
        let index = Index::open_in_memory().expect("open");
        // First load: schema has property "summary"
        let mut schema = empty_schema();
        schema.types.get_mut(&EntityType::Note).unwrap().properties = vec![Property::Text(
            TextProperty {
                key: "summary".into(),
                name: "Summary".into(),
                description: None,
                required: false,
                default: None,
            },
        )];
        let mut n = Note {
            name: "X".into(),
            tags: vec!["eduport-type/note".into()],
            custom: Default::default(),
        };
        n.custom
            .insert("summary".into(), serde_yaml::Value::String("a".into()));
        n.custom
            .insert("note".into(), serde_yaml::Value::String("b".into()));
        upsert_entity(
            index.conn(),
            "x",
            std::path::Path::new("x.md"),
            0,
            &Entity::Note(n),
            "",
            Some(&schema),
        )
        .unwrap();

        // After: schema declares "note" instead. Reindex should
        // replace "summary" with "note" in the properties table.
        schema.types.get_mut(&EntityType::Note).unwrap().properties = vec![Property::Text(
            TextProperty {
                key: "note".into(),
                name: "Note".into(),
                description: None,
                required: false,
                default: None,
            },
        )];
        let n_reindexed = reindex_all_properties(index.conn(), &schema).unwrap();
        assert_eq!(n_reindexed, 1);

        let key: String = index
            .conn()
            .query_row(
                "SELECT key FROM properties WHERE file_id = 'x'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(key, "note");
    }
}
