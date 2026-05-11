//! FTS search adapter — wraps `vaultdb_fts::search` and extends each
//! hit with the eduport entity type (parsed from the `eduport-type/<value>`
//! tag that every entity carries).
//!
//! vaultdb-fts is type-agnostic on purpose; eduport reconstructs the
//! type from tags so the existing frontend `SearchHit` shape (which
//! has an `entity_type` field) doesn't need to change.

use rusqlite::Connection;

use crate::entity::types::EDUPORT_TYPE_PREFIX;

use super::IndexError;

/// One FTS5 hit, eduport-shaped — adds `entity_type` on top of the
/// generic vaultdb-fts result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    pub file_id: String,
    pub entity_type: String,
    pub name: String,
    pub snippet: String,
}

/// Run an FTS5 MATCH against the eduport index, optionally restricted
/// to entities that carry every tag in `tags`.
///
/// Hits missing the `eduport-type/<value>` discriminator (shouldn't
/// happen for entities written through this crate, but possible for
/// hand-edited files) are dropped — they're not addressable through
/// the eduport URL scheme anyway.
pub fn search_fts(
    conn: &Connection,
    query: &str,
    limit: usize,
    tags: &[&str],
) -> Result<Vec<SearchHit>, IndexError> {
    let raw = vaultdb_fts::search(conn, query, limit, tags)?;
    Ok(raw
        .into_iter()
        .filter_map(|hit| {
            let entity_type = hit
                .tags
                .iter()
                .find_map(|t| t.strip_prefix(EDUPORT_TYPE_PREFIX).map(String::from))?;
            Some(SearchHit {
                file_id: hit.file_id,
                entity_type,
                name: hit.name,
                snippet: hit.snippet,
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::super::Index;
    use super::super::writer::upsert_entity;
    use super::*;
    use crate::entity::{Entity, Note};

    fn note_with_tags(name: &str, extra_tags: &[&str]) -> Entity {
        let mut n = Note {
            name: name.into(),
            tags: vec!["eduport-type/note".into()],
            custom: Default::default(),
        };
        for t in extra_tags {
            n.tags.push((*t).into());
        }
        Entity::Note(n)
    }

    #[test]
    fn search_fts_finds_body_match_and_carries_type() {
        let index = Index::open_in_memory().unwrap();
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
        let hits = search_fts(index.conn(), "fox", 10, &[]).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "Alpha");
        assert_eq!(hits[0].entity_type, "note");
    }

    #[test]
    fn search_fts_intersects_with_tag_filter() {
        let index = Index::open_in_memory().unwrap();
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
}
