//! Eduport-shaped wrappers over `vaultdb_fts::{upsert, delete,
//! record_parse_error, clear_parse_error}`.
//!
//! Builds a vaultdb-fts [`Document`] from an eduport [`Entity`] +
//! optional schema (for the `custom_text` FTS5 column). The shared
//! crate doesn't know about eduport's entity types or schemas — those
//! details get folded in here.

use std::path::Path;

use rusqlite::Connection;

use crate::entity::Entity;
use crate::schema::{PropertyKind, Schema};

use super::IndexError;

/// Update or insert the FTS row for a single entity.
///
/// `file_id` is the filename stem. `mtime_ns` lets reconcile skip
/// up-to-date files. `body` is the markdown after the frontmatter.
/// `schema` is optional — when present, custom-property `Text` /
/// `Url` values are folded into the FTS5 `custom_text` column so the
/// command palette can find them.
pub fn upsert_entity(
    conn: &Connection,
    file_id: &str,
    path: &Path,
    mtime_ns: i64,
    entity: &Entity,
    body: &str,
    schema: Option<&Schema>,
) -> Result<(), IndexError> {
    let custom_text = match schema {
        Some(s) => custom_text_for_fts5(entity, s),
        None => String::new(),
    };
    vaultdb_fts::upsert(
        conn,
        &vaultdb_fts::Document {
            file_id,
            path,
            mtime_ns,
            body,
            name: entity.name(),
            tags: entity.tags(),
            custom_text: &custom_text,
        },
    )?;
    Ok(())
}

pub fn delete_entity(conn: &Connection, file_id: &str) -> Result<(), IndexError> {
    vaultdb_fts::delete(conn, file_id)?;
    Ok(())
}

pub fn record_parse_error(
    conn: &Connection,
    path: &str,
    message: &str,
) -> Result<(), IndexError> {
    vaultdb_fts::record_parse_error(conn, path, message)?;
    Ok(())
}

pub fn clear_parse_error(conn: &Connection, path: &str) -> Result<(), IndexError> {
    vaultdb_fts::clear_parse_error(conn, path)?;
    Ok(())
}

pub(crate) fn custom_text_for_fts5(entity: &Entity, schema: &Schema) -> String {
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
    use crate::schema::{Property, TextProperty, empty_schema};
    use crate::EntityType;

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
        let index = Index::open_in_memory().unwrap();
        let entity = note("Hello", &["greeting"]);
        upsert_entity(
            index.conn(),
            "hello-1234",
            Path::new("notes/hello-1234.md"),
            42,
            &entity,
            "world body",
            None,
        )
        .unwrap();
        let (file_id, name, mtime_ns): (String, String, i64) = index
            .conn()
            .query_row(
                "SELECT file_id, name, mtime_ns FROM entities",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(file_id, "hello-1234");
        assert_eq!(name, "Hello");
        assert_eq!(mtime_ns, 42);
    }

    #[test]
    fn delete_cascades_to_fts() {
        let index = Index::open_in_memory().unwrap();
        let entity = note("Y", &["t1"]);
        upsert_entity(
            index.conn(),
            "y",
            Path::new("y.md"),
            0,
            &entity,
            "body",
            None,
        )
        .unwrap();
        delete_entity(index.conn(), "y").unwrap();
        let n: i64 = index
            .conn()
            .query_row("SELECT COUNT(*) FROM entities", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn upsert_with_schema_populates_fts_custom_text() {
        let index = Index::open_in_memory().unwrap();
        let mut schema = empty_schema();
        schema.types.get_mut(&EntityType::Note).unwrap().properties =
            vec![Property::Text(TextProperty {
                key: "summary".into(),
                name: "Summary".into(),
                description: None,
                required: false,
                is_builtin: false,
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
            Path::new("howdy.md"),
            0,
            &entity,
            "",
            Some(&schema),
        )
        .unwrap();
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
