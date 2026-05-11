//! Read-side accessor for the FTS5 search index.
//!
//! This used to also carry list / filter / aggregate functions that
//! queried a bespoke `properties` + `entity_tags` SQLite cache. Those
//! moved to `Vault::query` (see `crate::query`) so eduport stops
//! re-implementing what vaultdb-core already exposes. What stays
//! here is full-text search — the one genuinely-unique value-add
//! eduport's index brings (vaultdb-core doesn't have FTS yet).

use std::collections::HashSet;

use rusqlite::Connection;

use super::IndexError;

/// One FTS5 hit: the entity summary plus an FTS5-generated snippet
/// of the matching body region (with `<<` / `>>` markers around the
/// matched terms — same shape the Python sidecar emits, so the
/// frontend snippet renderer doesn't need to change).
#[derive(Debug, Clone, PartialEq)]
pub struct SearchHit {
    pub file_id: String,
    pub entity_type: String,
    pub name: String,
    pub snippet: String,
}

/// Run an FTS5 `MATCH` query and return up to `limit` hits, optionally
/// intersected with a tag filter (an entity must carry **every** tag
/// in `tags` to be returned).
///
/// The query string is passed through to FTS5 unchanged — callers
/// must escape any FTS5-special characters themselves. Hits come back
/// in FTS5's default rank order (best match first).
///
/// Tag intersection used to live in a sibling `entity_tags` SQLite
/// table; that table is gone now. We read the FTS5 row's own `tags`
/// column (space-joined tag list from the writer) and post-filter
/// in Rust. Tag values can't legitimately contain whitespace, so
/// the split is unambiguous.
pub fn search_fts(
    conn: &Connection,
    query: &str,
    limit: usize,
    tags: &[&str],
) -> Result<Vec<SearchHit>, IndexError> {
    // Without a tag filter, the SQL LIMIT bounds the work directly.
    // With one, we overscan by a small multiple so the Rust-side
    // filter has enough candidates to satisfy `limit` after intersection.
    let scan_limit = if tags.is_empty() { limit } else { limit * 4 };

    let mut stmt = conn.prepare(
        "SELECT e.file_id, e.type, e.name, \
                snippet(entities_fts, 0, '<<', '>>', '...', 16) AS snippet, \
                entities_fts.tags AS row_tags \
         FROM entities_fts \
         JOIN entities e ON e.rowid = entities_fts.rowid \
         WHERE entities_fts MATCH ?1 \
         LIMIT ?2",
    )?;
    let rows = stmt.query_map(
        rusqlite::params![query, scan_limit as i64],
        |r| -> rusqlite::Result<(SearchHit, String)> {
            Ok((
                SearchHit {
                    file_id: r.get(0)?,
                    entity_type: r.get(1)?,
                    name: r.get(2)?,
                    snippet: r.get(3)?,
                },
                r.get::<_, String>(4)?,
            ))
        },
    )?;

    let required: HashSet<&str> = tags.iter().copied().collect();
    let mut out: Vec<SearchHit> = Vec::new();
    for row in rows {
        let (hit, row_tags) = row?;
        if !required.is_empty() {
            let row_set: HashSet<&str> = row_tags.split_whitespace().collect();
            if !required.iter().all(|t| row_set.contains(t)) {
                continue;
            }
        }
        out.push(hit);
        if out.len() >= limit {
            break;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::super::Index;
    use super::super::writer::upsert_entity;
    use super::*;
    use crate::entity::{Entity, Note};

    fn note_with_tags(name: &str, tags: &[&str]) -> Entity {
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
    fn search_fts_finds_body_match() {
        let index = Index::open_in_memory().expect("open");
        upsert_entity(
            index.conn(),
            "n1",
            std::path::Path::new("n1.md"),
            0,
            &note_with_tags("Alpha", &[]),
            "the quick brown fox",
            None,
        )
        .unwrap();
        upsert_entity(
            index.conn(),
            "n2",
            std::path::Path::new("n2.md"),
            0,
            &note_with_tags("Beta", &[]),
            "lazy dog",
            None,
        )
        .unwrap();
        let hits = search_fts(index.conn(), "fox", 10, &[]).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "Alpha");
        assert!(hits[0].snippet.contains("fox"));
    }

    #[test]
    fn search_fts_intersects_with_tag_filter() {
        let index = Index::open_in_memory().expect("open");
        upsert_entity(
            index.conn(),
            "n1",
            std::path::Path::new("n1.md"),
            0,
            &note_with_tags("Alpha", &["draft"]),
            "matching body",
            None,
        )
        .unwrap();
        upsert_entity(
            index.conn(),
            "n2",
            std::path::Path::new("n2.md"),
            0,
            &note_with_tags("Beta", &["published"]),
            "matching body",
            None,
        )
        .unwrap();
        let hits = search_fts(index.conn(), "matching", 10, &["draft"]).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "Alpha");
    }

    #[test]
    fn search_fts_requires_all_tags() {
        let index = Index::open_in_memory().expect("open");
        upsert_entity(
            index.conn(),
            "a",
            std::path::Path::new("a.md"),
            0,
            &note_with_tags("Alpha", &["draft", "japan"]),
            "matching body",
            None,
        )
        .unwrap();
        upsert_entity(
            index.conn(),
            "b",
            std::path::Path::new("b.md"),
            0,
            &note_with_tags("Beta", &["draft"]),
            "matching body",
            None,
        )
        .unwrap();
        let hits = search_fts(index.conn(), "matching", 10, &["draft", "japan"]).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "Alpha");
    }
}
