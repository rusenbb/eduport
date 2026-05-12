//! Bridge from eduport's filter request shape to vaultdb's query AST.
//!
//! This is the seam where eduport stops re-implementing what
//! vaultdb-core already offers. Eduport had its own SQLite-backed
//! `properties` table and a hand-rolled SQL JOIN builder for filter
//! evaluation. vaultdb has the same surface natively: `Vault::query`
//! takes an `Expr`/`Predicate` AST and returns `Record`s straight off
//! disk. We map the frontend's `PropertyFilters`-shaped request onto
//! that AST and delegate everything else.
//!
//! ## What this module is responsible for
//!
//! - Translating eduport's "filter chips" into `Predicate` /
//!   `CompareOp` clauses combined with `And`.
//! - Pinning the result set to one [`EntityType`] via the
//!   `eduport-type/<value>` tag, so callers never have to remember
//!   to add that themselves.
//! - Sort-key + ascending/descending normalisation.
//! - Returning vaultdb [`Record`]s in a shape eduport's existing
//!   list-row response type can consume.
//!
//! ## Predicate-shape choices
//!
//! - **Text filters → [`Predicate::Equals`]**. Matches eduport's
//!   prior SQL behaviour (`value_text = ?`). The chip placeholder
//!   says "contains…" but the historical semantics were exact —
//!   changing that is a UX decision, not part of this migration.
//! - **Tag filters → [`Predicate::Contains`] on `tags`**. vaultdb
//!   special-cases `Contains` over a [`Value::List`] as list-member
//!   equality (see `crate::filter::evaluate_predicate`), which is
//!   exactly the semantics tags want.
//! - **Numeric range → two [`Predicate::Compare`] clauses** joined
//!   by `And`. vaultdb's `Compare` does cross-numeric coercion so
//!   `i64`-vs-`f64` doesn't matter at the storage layer.
//! - **Date range → string-compared [`Predicate::Compare`]**. We
//!   only support ISO 8601 (`YYYY-MM-DD`); the frontend's `<input
//!   type="date">` is the only path that fills these in, so the
//!   constraint is enforced upstream.

use std::collections::BTreeMap;

use vaultdb_core::{CompareOp, Expr, Predicate, Query, Record, SortKey, Value};

use crate::EntityType;
use crate::entity::types::EDUPORT_TYPE_PREFIX;

/// Frontend-shaped filter request, deserialised by the Tauri layer.
/// Mirrors the JSON the frontend's `filterEntitiesByProperties` posts.
#[derive(Debug, Clone)]
pub struct FilterInput<'a> {
    pub text: &'a BTreeMap<String, String>,
    pub num: &'a BTreeMap<String, (Option<f64>, Option<f64>)>,
    pub date: &'a BTreeMap<String, (Option<String>, Option<String>)>,
    pub tags: &'a [&'a str],
    pub sort_key: Option<&'a str>,
    /// `"asc"` or `"desc"`. Anything else falls through to ascending.
    pub sort_dir: &'a str,
}

/// Build a vaultdb [`Query`] that asks for all entities of `entity_type`
/// matching `filters`. The result set is pinned by the
/// `eduport-type/<value>` tag.
///
/// `folder` is empty so the query scans the vault root non-recursively
/// — the canonical eduport layout (every entity is a `.md` at the
/// root; `notes/` and `attachments/` are user space).
pub fn query_for_filter(entity_type: EntityType, filters: &FilterInput<'_>) -> Query {
    let type_tag = format!("{}{}", EDUPORT_TYPE_PREFIX, entity_type.as_str());

    let mut clauses: Vec<Expr> = Vec::new();

    // Type discriminator.
    clauses.push(Expr::Predicate(Predicate::Contains {
        field: "tags".into(),
        value: Value::String(type_tag),
    }));

    // Tag intersection — one Contains per requested tag.
    for tag in filters.tags {
        clauses.push(Expr::Predicate(Predicate::Contains {
            field: "tags".into(),
            value: Value::String((*tag).to_string()),
        }));
    }

    // Text filters. Empty values are the "(any)" placeholder; drop
    // them so they don't add a `value = ""` constraint that matches
    // nothing.
    for (key, value) in filters.text.iter() {
        if value.is_empty() {
            continue;
        }
        clauses.push(Expr::Predicate(Predicate::Equals {
            field: key.clone(),
            value: Value::String(value.clone()),
        }));
    }

    // Numeric ranges. Each populated bound becomes a Compare clause.
    for (key, (lo, hi)) in filters.num.iter() {
        if let Some(lo) = lo {
            clauses.push(Expr::Predicate(Predicate::Compare {
                field: key.clone(),
                op: CompareOp::Ge,
                value: Value::Float(*lo),
            }));
        }
        if let Some(hi) = hi {
            clauses.push(Expr::Predicate(Predicate::Compare {
                field: key.clone(),
                op: CompareOp::Le,
                value: Value::Float(*hi),
            }));
        }
    }

    // Date ranges. ISO-8601 strings sort lexicographically the same
    // as chronologically, so Compare on `String` is correct here.
    for (key, (lo, hi)) in filters.date.iter() {
        if let Some(lo) = lo {
            clauses.push(Expr::Predicate(Predicate::Compare {
                field: key.clone(),
                op: CompareOp::Ge,
                value: Value::String(lo.clone()),
            }));
        }
        if let Some(hi) = hi {
            clauses.push(Expr::Predicate(Predicate::Compare {
                field: key.clone(),
                op: CompareOp::Le,
                value: Value::String(hi.clone()),
            }));
        }
    }

    let filter = if clauses.len() == 1 {
        Some(clauses.into_iter().next().unwrap())
    } else {
        Some(Expr::And(clauses))
    };

    let sort = filters.sort_key.map(|field| SortKey {
        field: field.to_string(),
        descending: filters.sort_dir == "desc",
    });

    Query {
        folder: String::new(),
        filter,
        select: None,
        sort,
        limit: None,
        recursive: false,
    }
}

/// Build a vaultdb [`Query`] that finds every entity whose `parent`
/// frontmatter field equals `parent_file_id`. Cross-type — does
/// **not** pin by `eduport-type/<value>`. Used by the page-hierarchy
/// "sub-pages" UI: a Person can be a sub-page of a University, a
/// Note can be a sub-page of an Application, etc.
///
/// Scans the vault root non-recursively, same as the type-pinned
/// query. Subfolders (`notes/`, `attachments/`) are user space and
/// aren't entity-shaped — they're correctly ignored.
pub fn query_for_children(parent_file_id: &str) -> Query {
    Query {
        folder: String::new(),
        filter: Some(Expr::Predicate(Predicate::Equals {
            field: "parent".into(),
            value: Value::String(parent_file_id.to_string()),
        })),
        select: None,
        sort: Some(SortKey {
            field: "name".into(),
            descending: false,
        }),
        limit: None,
        recursive: false,
    }
}

/// One row in the entity-list view, derived from a vaultdb [`Record`].
/// Shaped to match the frontend's `EntityListItem`.
#[derive(Debug, Clone)]
pub struct EntitySummaryView {
    pub file_id: String,
    pub entity_type: String,
    pub name: String,
    pub path: String,
}

impl EntitySummaryView {
    /// Project a vaultdb [`Record`] down to the shape the entity
    /// list view consumes. Returns `None` for records whose
    /// frontmatter doesn't look like an eduport entity at all
    /// (no `eduport-type/<value>` tag, or no `name`). Callers
    /// silently drop those — the watcher / reconcile path is what
    /// surfaces malformed entity files.
    pub fn from_record(r: &Record) -> Option<Self> {
        let file_id = r.path.file_stem()?.to_str()?.to_string();
        let name = match r.fields.get("name") {
            Some(Value::String(s)) => s.clone(),
            _ => return None,
        };
        let entity_type = match r.fields.get("tags") {
            Some(Value::List(items)) => items.iter().find_map(|v| {
                if let Value::String(t) = v
                    && let Some(rest) = t.strip_prefix(EDUPORT_TYPE_PREFIX)
                {
                    Some(rest.to_string())
                } else {
                    None
                }
            })?,
            _ => return None,
        };
        Some(Self {
            file_id,
            entity_type,
            name,
            path: r.path.to_string_lossy().into_owned(),
        })
    }
}

/// Count distinct values of `key` across all entities of `entity_type`.
/// Used by the sidebar property-chip section.
///
/// `(value, count)` pairs are returned descending by count, then
/// ascending by value for stability. The cost is one vault.query()
/// call (which streams records off disk) plus an O(N) aggregation
/// pass — same complexity as the previous SQL `GROUP BY`.
pub fn value_counts_for(records: &[Record], key: &str) -> Vec<(String, i64)> {
    let mut buckets: BTreeMap<String, i64> = BTreeMap::new();
    for r in records {
        match r.fields.get(key) {
            Some(Value::String(s)) => {
                *buckets.entry(s.clone()).or_insert(0) += 1;
            }
            Some(Value::List(items)) => {
                for v in items {
                    if let Value::String(s) = v {
                        *buckets.entry(s.clone()).or_insert(0) += 1;
                    }
                }
            }
            _ => {}
        }
    }
    let mut out: Vec<(String, i64)> = buckets.into_iter().collect();
    out.sort_by(|(av, ac), (bv, bc)| bc.cmp(ac).then_with(|| av.cmp(bv)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EntityType;

    #[test]
    fn query_pins_to_entity_type_via_tag() {
        let text = BTreeMap::new();
        let num = BTreeMap::new();
        let date = BTreeMap::new();
        let tags: &[&str] = &[];
        let input = FilterInput {
            text: &text,
            num: &num,
            date: &date,
            tags,
            sort_key: None,
            sort_dir: "asc",
        };
        let q = query_for_filter(EntityType::University, &input);
        match q.filter.as_ref().unwrap() {
            Expr::Predicate(Predicate::Contains { field, value }) => {
                assert_eq!(field, "tags");
                assert_eq!(value, &Value::String("eduport-type/university".into()));
            }
            other => panic!("expected single tag predicate, got {other:?}"),
        }
    }

    #[test]
    fn empty_text_value_is_dropped() {
        let mut text = BTreeMap::new();
        text.insert("country".into(), String::new());
        text.insert("city".into(), "Tokyo".into());
        let num = BTreeMap::new();
        let date = BTreeMap::new();
        let input = FilterInput {
            text: &text,
            num: &num,
            date: &date,
            tags: &[],
            sort_key: None,
            sort_dir: "asc",
        };
        let q = query_for_filter(EntityType::University, &input);
        let clauses = match q.filter.unwrap() {
            Expr::And(c) => c,
            other => panic!("expected And, got {other:?}"),
        };
        // Type tag + city, nothing for empty country.
        assert_eq!(clauses.len(), 2);
    }

    #[test]
    fn tag_filter_is_list_membership() {
        let text = BTreeMap::new();
        let num = BTreeMap::new();
        let date = BTreeMap::new();
        let tags = ["japan", "ylsy"];
        let tag_refs: Vec<&str> = tags.to_vec();
        let input = FilterInput {
            text: &text,
            num: &num,
            date: &date,
            tags: &tag_refs,
            sort_key: None,
            sort_dir: "asc",
        };
        let q = query_for_filter(EntityType::University, &input);
        let clauses = match q.filter.unwrap() {
            Expr::And(c) => c,
            other => panic!("expected And, got {other:?}"),
        };
        // type tag + 2 user tags
        assert_eq!(clauses.len(), 3);
        for c in &clauses[1..] {
            assert!(
                matches!(c, Expr::Predicate(Predicate::Contains { field, .. }) if field == "tags")
            );
        }
    }

    #[test]
    fn num_range_emits_compare_ge_and_le() {
        let text = BTreeMap::new();
        let mut num = BTreeMap::new();
        num.insert("year".into(), (Some(2020.0), Some(2024.0)));
        let date = BTreeMap::new();
        let input = FilterInput {
            text: &text,
            num: &num,
            date: &date,
            tags: &[],
            sort_key: None,
            sort_dir: "asc",
        };
        let q = query_for_filter(EntityType::Program, &input);
        let clauses = match q.filter.unwrap() {
            Expr::And(c) => c,
            other => panic!("expected And, got {other:?}"),
        };
        let ops: Vec<CompareOp> = clauses
            .iter()
            .filter_map(|c| match c {
                Expr::Predicate(Predicate::Compare { op, .. }) => Some(*op),
                _ => None,
            })
            .collect();
        assert_eq!(ops, vec![CompareOp::Ge, CompareOp::Le]);
    }

    #[test]
    fn sort_key_propagates_with_direction() {
        let text = BTreeMap::new();
        let num = BTreeMap::new();
        let date = BTreeMap::new();
        let input = FilterInput {
            text: &text,
            num: &num,
            date: &date,
            tags: &[],
            sort_key: Some("country"),
            sort_dir: "desc",
        };
        let q = query_for_filter(EntityType::University, &input);
        let sort = q.sort.unwrap();
        assert_eq!(sort.field, "country");
        assert!(sort.descending);
    }
}
