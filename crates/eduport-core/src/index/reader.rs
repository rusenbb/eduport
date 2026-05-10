//! Read-side accessors for the FTS5 index.
//!
//! Free functions taking `&Connection` so callers can compose with
//! their own transactions. The reader returns plain DTOs (no SQL
//! types leaking out); the writer's column-shape choices stay
//! private to [`super::writer`].

use rusqlite::{Connection, params_from_iter};

use crate::EntityType;

use super::IndexError;

/// Lightweight summary of an entity row, suitable for list views and
/// search results. Carries only the fields needed for "show me a
/// list" — the full entity comes from [`crate::EntityStore::find_by_name`]
/// when the user clicks through.
#[derive(Debug, Clone, PartialEq)]
pub struct EntitySummary {
    pub file_id: String,
    pub entity_type: String,
    pub name: String,
    pub path: String,
    pub mtime_ns: i64,
}

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

/// Aggregated property-value count. Used by the sidebar chip
/// aggregation (e.g. "Programs in Stanford: 12, MIT: 8").
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyValueCount {
    /// The property kind ("text", "number", "single-select", ...).
    pub property_type: String,
    /// String form of the value. For checkboxes this is "true"/"false";
    /// for multi-select the caller iterates one entry per chosen value.
    pub value: String,
    pub count: i64,
}

/// List entities, optionally filtered by type and a tag set. When
/// `tags` is non-empty an entity must carry *all* given tags
/// (intersection semantics — same as the Python sidecar).
///
/// Sorted by `name` ascending; the caller is expected to apply any
/// view-specific sort post-fetch.
pub fn list_entities(
    conn: &Connection,
    entity_type: Option<EntityType>,
    tags: &[&str],
) -> Result<Vec<EntitySummary>, IndexError> {
    let mut where_clauses: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(t) = entity_type {
        where_clauses.push("type = ?".into());
        params.push(Box::new(t.as_str().to_string()));
    }
    if !tags.is_empty() {
        let placeholders: Vec<&str> = std::iter::repeat_n("?", tags.len()).collect();
        where_clauses.push(format!(
            "file_id IN (\
               SELECT file_id FROM entity_tags \
               WHERE tag IN ({}) \
               GROUP BY file_id HAVING COUNT(DISTINCT tag) = ?\
             )",
            placeholders.join(", ")
        ));
        for tag in tags {
            params.push(Box::new(tag.to_string()));
        }
        params.push(Box::new(tags.len() as i64));
    }

    let mut sql = String::from("SELECT file_id, type, name, path, mtime_ns FROM entities");
    if !where_clauses.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&where_clauses.join(" AND "));
    }
    sql.push_str(" ORDER BY name");

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params.iter().map(|b| b.as_ref())), |r| {
        Ok(EntitySummary {
            file_id: r.get(0)?,
            entity_type: r.get(1)?,
            name: r.get(2)?,
            path: r.get(3)?,
            mtime_ns: r.get(4)?,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// Run an FTS5 `MATCH` query and return up to `limit` hits, optionally
/// intersected with a tag filter (same semantics as [`list_entities`]).
///
/// The query string is passed through to FTS5 unchanged — callers are
/// responsible for escaping/quoting special characters as needed.
/// Returns hits in FTS5's default rank order (best match first).
pub fn search_fts(
    conn: &Connection,
    query: &str,
    limit: usize,
    tags: &[&str],
) -> Result<Vec<SearchHit>, IndexError> {
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    params.push(Box::new(query.to_string()));

    let tag_clause = if tags.is_empty() {
        String::new()
    } else {
        let placeholders: Vec<&str> = std::iter::repeat_n("?", tags.len()).collect();
        for tag in tags {
            params.push(Box::new(tag.to_string()));
        }
        params.push(Box::new(tags.len() as i64));
        format!(
            " AND e.file_id IN (\
               SELECT file_id FROM entity_tags \
               WHERE tag IN ({}) \
               GROUP BY file_id HAVING COUNT(DISTINCT tag) = ?\
             )",
            placeholders.join(", ")
        )
    };
    params.push(Box::new(limit as i64));

    let sql = format!(
        "SELECT e.file_id, e.type, e.name, \
                snippet(entities_fts, 0, '<<', '>>', '...', 16) AS snippet \
         FROM entities_fts \
         JOIN entities e ON e.rowid = entities_fts.rowid \
         WHERE entities_fts MATCH ?{tag_clause} \
         LIMIT ?"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params.iter().map(|b| b.as_ref())), |r| {
        Ok(SearchHit {
            file_id: r.get(0)?,
            entity_type: r.get(1)?,
            name: r.get(2)?,
            snippet: r.get(3)?,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// Filter parameters for [`filter_entities_by_properties`]. Each map
/// is `key → constraint`; an entity matches when every constraint in
/// every map matches one of its property rows.
#[derive(Debug, Clone, Default)]
pub struct PropertyFilter<'a> {
    pub text_equals: &'a [(&'a str, &'a str)],
    pub num_range: &'a [(&'a str, Option<f64>, Option<f64>)],
    pub date_range: &'a [(&'a str, Option<&'a str>, Option<&'a str>)],
    pub sort_key: Option<&'a str>,
    /// `"asc"` or `"desc"`. Anything else is treated as `"asc"`.
    pub sort_dir: &'a str,
}

/// Filter and sort entities by custom-property values. Mirrors the
/// Python sidecar's `filter_entities_by_properties` — used by view
/// rendering to honour saved-view filters/sorts.
///
/// When `sort_key` is set, sorts on
/// `COALESCE(value_num, value_date, value_text)` so a single column
/// covers all the property kinds the user might want to sort on. Ties
/// break on entity name ascending.
pub fn filter_entities_by_properties(
    conn: &Connection,
    entity_type: EntityType,
    filter: &PropertyFilter<'_>,
) -> Result<Vec<EntitySummary>, IndexError> {
    // Param ordering matters: positional placeholders bind in textual
    // SQL order, so we collect join params and where params into
    // separate buckets and concat them in `join → where` order at the
    // end. This was a real bug at one point — putting `e.type` first
    // bound it to a JOIN's `?` and silently produced empty results.
    let mut join_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    let mut where_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    let mut joins: Vec<String> = Vec::new();
    let mut where_clauses: Vec<String> = vec!["e.type = ?".into()];
    where_params.push(Box::new(entity_type.as_str().to_string()));

    // Text-equality joins. Each filter gets a unique alias so multiple
    // text constraints on different keys don't collide on the same
    // join row.
    for (i, (key, value)) in filter.text_equals.iter().enumerate() {
        let alias = format!("pt{i}");
        joins.push(format!(
            "JOIN properties {alias} ON {alias}.file_id = e.file_id AND {alias}.key = ?"
        ));
        join_params.push(Box::new((*key).to_string()));
        where_clauses.push(format!("{alias}.value_text = ?"));
        where_params.push(Box::new((*value).to_string()));
    }

    for (i, (key, lo, hi)) in filter.num_range.iter().enumerate() {
        let alias = format!("pn{i}");
        joins.push(format!(
            "JOIN properties {alias} ON {alias}.file_id = e.file_id AND {alias}.key = ?"
        ));
        join_params.push(Box::new((*key).to_string()));
        if let Some(lo) = lo {
            where_clauses.push(format!("{alias}.value_num >= ?"));
            where_params.push(Box::new(*lo));
        }
        if let Some(hi) = hi {
            where_clauses.push(format!("{alias}.value_num <= ?"));
            where_params.push(Box::new(*hi));
        }
    }

    for (i, (key, lo, hi)) in filter.date_range.iter().enumerate() {
        let alias = format!("pd{i}");
        joins.push(format!(
            "JOIN properties {alias} ON {alias}.file_id = e.file_id AND {alias}.key = ?"
        ));
        join_params.push(Box::new((*key).to_string()));
        if let Some(lo) = lo {
            where_clauses.push(format!("{alias}.value_date >= ?"));
            where_params.push(Box::new((*lo).to_string()));
        }
        if let Some(hi) = hi {
            where_clauses.push(format!("{alias}.value_date <= ?"));
            where_params.push(Box::new((*hi).to_string()));
        }
    }

    let mut sql = String::from("SELECT e.file_id, e.type, e.name, e.path, e.mtime_ns FROM entities e ");
    if !joins.is_empty() {
        sql.push_str(&joins.join(" "));
        sql.push(' ');
    }

    if let Some(sort_key) = filter.sort_key {
        sql.push_str("LEFT JOIN properties ps ON ps.file_id = e.file_id AND ps.key = ? ");
        join_params.push(Box::new(sort_key.to_string()));
    }

    sql.push_str("WHERE ");
    sql.push_str(&where_clauses.join(" AND "));

    // Splice into a single param list in the order the placeholders
    // appear in the final SQL: joins first (textually before WHERE),
    // then where params.
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::with_capacity(
        join_params.len() + where_params.len(),
    );
    params.extend(join_params);
    params.extend(where_params);

    if filter.sort_key.is_some() {
        let direction = if filter.sort_dir.eq_ignore_ascii_case("desc") {
            "DESC"
        } else {
            "ASC"
        };
        sql.push_str(&format!(
            " ORDER BY COALESCE(ps.value_num, ps.value_date, ps.value_text) {direction}, e.name ASC"
        ));
    } else {
        sql.push_str(" ORDER BY e.name");
    }

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params.iter().map(|b| b.as_ref())), |r| {
        Ok(EntitySummary {
            file_id: r.get(0)?,
            entity_type: r.get(1)?,
            name: r.get(2)?,
            path: r.get(3)?,
            mtime_ns: r.get(4)?,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// Aggregate property-value counts for the sidebar chip view.
///
/// For multi-select properties, each chosen value contributes its own
/// count entry — i.e. an entity tagged `["A", "B"]` adds 1 to both
/// `A` and `B`. Returned entries are sorted by descending count, then
/// ascending value.
pub fn property_value_counts(
    conn: &Connection,
    entity_type: EntityType,
    key: &str,
) -> Result<Vec<PropertyValueCount>, IndexError> {
    let mut stmt = conn.prepare(
        "SELECT p.type, p.value_text, p.value_num, p.value_multi, COUNT(*) AS count \
         FROM properties p \
         JOIN entities e ON e.file_id = p.file_id \
         WHERE e.type = ?1 AND p.key = ?2 \
         GROUP BY p.type, p.value_text, p.value_num, p.value_multi",
    )?;
    let rows = stmt
        .query_map(params![entity_type.as_str(), key], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, Option<f64>>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, i64>(4)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    // Aggregate into a HashMap so we can fold multi-select rows that
    // overlap with single-select rows on the same value.
    let mut buckets: std::collections::HashMap<(String, String), i64> =
        std::collections::HashMap::new();
    for (kind, text, num, multi, count) in rows {
        match kind.as_str() {
            "checkbox" => {
                let v = if num.unwrap_or(0.0) == 1.0 { "true" } else { "false" };
                *buckets.entry(("checkbox".into(), v.into())).or_insert(0) += count;
            }
            "multi-select" => {
                let Some(json) = multi else { continue };
                let Ok(values) = serde_json::from_str::<Vec<String>>(&json) else {
                    continue;
                };
                for v in values {
                    *buckets.entry(("multi-select".into(), v)).or_insert(0) += count;
                }
            }
            _ => {
                if let Some(v) = text {
                    *buckets.entry((kind, v)).or_insert(0) += count;
                }
            }
        }
    }

    let mut out: Vec<PropertyValueCount> = buckets
        .into_iter()
        .map(|((property_type, value), count)| PropertyValueCount {
            property_type,
            value,
            count,
        })
        .collect();
    out.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.value.cmp(&b.value)));
    Ok(out)
}

// rusqlite doesn't re-export `params!` from inside this module unless
// imported; do it once and reuse.
use rusqlite::params;

#[cfg(test)]
mod tests {
    use super::super::Index;
    use super::super::writer::upsert_entity;
    use super::*;
    use crate::entity::{Entity, Note};
    use crate::entity_type::EntityType;
    use crate::schema::{Property, TextProperty, empty_schema};

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
    fn list_entities_filters_by_type() {
        let index = Index::open_in_memory().expect("open");
        for (i, name) in ["Alpha", "Beta", "Gamma"].iter().enumerate() {
            upsert_entity(
                index.conn(),
                &format!("n{i}"),
                std::path::Path::new(&format!("n{i}.md")),
                i as i64,
                &note_with_tags(name, &[]),
                "",
                None,
            )
            .unwrap();
        }
        let all = list_entities(index.conn(), None, &[]).unwrap();
        assert_eq!(all.len(), 3);
        let notes = list_entities(index.conn(), Some(EntityType::Note), &[]).unwrap();
        assert_eq!(notes.len(), 3);
        let unis = list_entities(index.conn(), Some(EntityType::University), &[]).unwrap();
        assert_eq!(unis.len(), 0);
    }

    #[test]
    fn list_entities_filter_by_tags_intersection() {
        let index = Index::open_in_memory().expect("open");
        upsert_entity(
            index.conn(),
            "n1",
            std::path::Path::new("n1.md"),
            0,
            &note_with_tags("A", &["foo", "bar"]),
            "",
            None,
        )
        .unwrap();
        upsert_entity(
            index.conn(),
            "n2",
            std::path::Path::new("n2.md"),
            0,
            &note_with_tags("B", &["foo"]),
            "",
            None,
        )
        .unwrap();
        upsert_entity(
            index.conn(),
            "n3",
            std::path::Path::new("n3.md"),
            0,
            &note_with_tags("C", &["bar"]),
            "",
            None,
        )
        .unwrap();

        let with_foo = list_entities(index.conn(), None, &["foo"]).unwrap();
        assert_eq!(with_foo.len(), 2);

        let with_foo_and_bar = list_entities(index.conn(), None, &["foo", "bar"]).unwrap();
        assert_eq!(with_foo_and_bar.len(), 1);
        assert_eq!(with_foo_and_bar[0].name, "A");
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
    fn property_filter_text_equals() {
        let index = Index::open_in_memory().expect("open");
        let mut schema = empty_schema();
        schema.types.get_mut(&EntityType::Note).unwrap().properties = vec![Property::Text(
            TextProperty {
                key: "country".into(),
                name: "Country".into(),
                description: None,
                required: false,
                default: None,
            },
        )];
        for (i, country) in ["USA", "UK", "USA"].iter().enumerate() {
            let mut n = Note {
                name: format!("n{i}"),
                tags: vec!["eduport-type/note".into()],
                custom: Default::default(),
            };
            n.custom.insert(
                "country".into(),
                serde_yaml::Value::String((*country).into()),
            );
            upsert_entity(
                index.conn(),
                &format!("n{i}"),
                std::path::Path::new(&format!("n{i}.md")),
                0,
                &Entity::Note(n),
                "",
                Some(&schema),
            )
            .unwrap();
        }
        let filter = PropertyFilter {
            text_equals: &[("country", "USA")],
            ..Default::default()
        };
        let result =
            filter_entities_by_properties(index.conn(), EntityType::Note, &filter).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn property_value_counts_aggregates_multi_select() {
        use crate::schema::{MultiSelectProperty, SelectOption};
        let index = Index::open_in_memory().expect("open");
        let mut schema = empty_schema();
        schema.types.get_mut(&EntityType::Note).unwrap().properties = vec![Property::MultiSelect(
            MultiSelectProperty {
                key: "topics".into(),
                name: "Topics".into(),
                description: None,
                required: false,
                options: vec![
                    SelectOption {
                        value: "rust".into(),
                        label: "Rust".into(),
                        color: Default::default(),
                    },
                    SelectOption {
                        value: "wasm".into(),
                        label: "WASM".into(),
                        color: Default::default(),
                    },
                ],
                default: None,
            },
        )];

        let topics_for = |topics: &[&str]| -> serde_yaml::Value {
            serde_yaml::Value::Sequence(
                topics
                    .iter()
                    .map(|s| serde_yaml::Value::String((*s).into()))
                    .collect(),
            )
        };

        let cases = [
            ("a", topics_for(&["rust"])),
            ("b", topics_for(&["rust", "wasm"])),
            ("c", topics_for(&["wasm"])),
        ];
        for (id, value) in cases {
            let mut n = Note {
                name: id.into(),
                tags: vec!["eduport-type/note".into()],
                custom: Default::default(),
            };
            n.custom.insert("topics".into(), value);
            upsert_entity(
                index.conn(),
                id,
                std::path::Path::new(&format!("{id}.md")),
                0,
                &Entity::Note(n),
                "",
                Some(&schema),
            )
            .unwrap();
        }

        let counts =
            property_value_counts(index.conn(), EntityType::Note, "topics").unwrap();
        let map: std::collections::HashMap<String, i64> =
            counts.into_iter().map(|c| (c.value, c.count)).collect();
        assert_eq!(map.get("rust"), Some(&2));
        assert_eq!(map.get("wasm"), Some(&2));
    }
}
