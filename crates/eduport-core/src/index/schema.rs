//! SQLite + FTS5 schema for eduport-core's search/filter index.
//!
//! The index is a derived cache: every row here is reproducible from the
//! markdown vault on disk (via [`crate::EntityStore`] and vaultdb-core's
//! parsed records). Treat the database file as a build artefact —
//! deleting it just forces a [`super::reconcile::reconcile`] on next
//! startup.
//!
//! ## What this index *is not*
//!
//! Two tables from the Python sidecar's schema are deliberately absent:
//!
//! - **`entity_links`** — vaultdb-core already keeps a live, parsed
//!   `LinkGraph` over the vault. Duplicating it in SQL gave us two
//!   sources of truth for backlinks and a reconcile bug surface. Look
//!   ups go through `Vault::link_graph()` now.
//! - **`checkboxes`** — task-line tracking is its own feature surface
//!   (touched by the Tasks entity body, not the FTS5 layer). It will
//!   come back in a separate module if/when needed; bundling it here
//!   conflated "search index" with "task model".
//!
//! Re-introducing either table is a schema-version bump (see
//! [`INDEX_SCHEMA_VERSION`]) and a migration that rebuilds from vault
//! state — never an opaque ALTER on the existing database.
//!
//! ## Migration policy
//!
//! Old databases are blown away wholesale on a version mismatch. The
//! index has no canonical state of its own — it's always a projection
//! of the vault — so a migration is just "drop and re-reconcile",
//! which the watcher layer triggers on startup. This is intentionally
//! simpler than column-level ALTERs: the cost of a full reconcile at
//! 1k entities is sub-second, and the saved complexity is large.

use rusqlite::Connection;

/// Bumped whenever the on-disk schema changes shape. A mismatch with
/// `PRAGMA user_version` triggers a full rebuild from vault state.
///
/// History
/// - **1**: initial Rust port (mirrors the Python sidecar's schema
///   version 2 minus `entity_links` and `checkboxes`).
pub const INDEX_SCHEMA_VERSION: i64 = 1;

/// DDL for the index. Idempotent — every statement uses
/// `IF NOT EXISTS` so it's safe to apply on a partially-initialised
/// database. The `entities_fts` virtual table uses the `unicode61`
/// tokenizer with diacritic folding so search matches "café" against
/// "cafe" (matches the Python sidecar's behaviour).
const DDL: &str = r#"
CREATE TABLE IF NOT EXISTS entities (
    file_id     TEXT PRIMARY KEY,
    type        TEXT NOT NULL,
    name        TEXT NOT NULL,
    path        TEXT NOT NULL,
    mtime_ns    INTEGER NOT NULL,
    body        TEXT NOT NULL,
    frontmatter TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(type);

CREATE TABLE IF NOT EXISTS entity_tags (
    file_id TEXT NOT NULL REFERENCES entities(file_id) ON DELETE CASCADE,
    tag     TEXT NOT NULL,
    PRIMARY KEY (file_id, tag)
);
CREATE INDEX IF NOT EXISTS idx_entity_tags_tag ON entity_tags(tag);

CREATE TABLE IF NOT EXISTS parse_errors (
    path        TEXT PRIMARY KEY,
    message     TEXT NOT NULL,
    occurred_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS properties (
    file_id     TEXT NOT NULL REFERENCES entities(file_id) ON DELETE CASCADE,
    key         TEXT NOT NULL,
    type        TEXT NOT NULL,
    value_text  TEXT,
    value_num   REAL,
    value_date  TEXT,
    value_multi TEXT,
    PRIMARY KEY (file_id, key)
);
CREATE INDEX IF NOT EXISTS idx_properties_key_text ON properties(key, value_text);
CREATE INDEX IF NOT EXISTS idx_properties_key_num  ON properties(key, value_num);
CREATE INDEX IF NOT EXISTS idx_properties_key_date ON properties(key, value_date);

CREATE VIRTUAL TABLE IF NOT EXISTS entities_fts USING fts5(
    body,
    name,
    tags,
    custom_text,
    tokenize="unicode61 remove_diacritics 2"
);
"#;

/// Outcome of a [`init_schema`] call. The `fts_rebuilt` flag tells the
/// caller "the FTS5 virtual table was reset (either by a version
/// migration or because it was missing); re-populate from `entities`
/// before returning to the user". Fresh databases return `false` —
/// they have nothing to repopulate yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InitOutcome {
    /// `true` iff the FTS5 virtual table was dropped during
    /// initialisation. The reconcile layer uses this to decide whether
    /// to force-reindex even when mtimes match the cached value.
    pub fts_rebuilt: bool,
}

/// Apply [`DDL`] and any version migrations.
///
/// The migration policy is "blow away the FTS5 table on version
/// downgrade-or-mismatch and let the caller repopulate". This is safe
/// because the FTS5 table is a derived projection of `entities` and
/// always rebuildable in O(N) where N is entity count (sub-second at
/// vault sizes we target — see [`super`]).
///
/// `entities` itself is preserved across migrations in this version;
/// future schema changes that touch `entities` columns will need a
/// proper migration block here (and a `INDEX_SCHEMA_VERSION` bump).
pub fn init_schema(conn: &Connection) -> rusqlite::Result<InitOutcome> {
    let current_version: i64 = conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap_or(0);

    let mut fts_rebuilt = false;
    if current_version != 0 && current_version != INDEX_SCHEMA_VERSION {
        // Old database — drop the FTS5 table; the new DDL will recreate
        // it with the current shape. The caller is expected to
        // re-populate via reconcile.
        conn.execute("DROP TABLE IF EXISTS entities_fts", [])?;
        fts_rebuilt = true;
    }

    conn.execute_batch(DDL)?;
    conn.pragma_update(None, "user_version", INDEX_SCHEMA_VERSION)?;
    Ok(InitOutcome { fts_rebuilt })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_in_memory() -> Connection {
        Connection::open_in_memory().expect("open in-memory sqlite")
    }

    #[test]
    fn fresh_database_initialises_cleanly() {
        let conn = open_in_memory();
        let outcome = init_schema(&conn).expect("init_schema");
        assert!(
            !outcome.fts_rebuilt,
            "fresh databases should not be flagged for FTS rebuild"
        );
        let version: i64 = conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, INDEX_SCHEMA_VERSION);
    }

    #[test]
    fn init_schema_is_idempotent() {
        let conn = open_in_memory();
        init_schema(&conn).expect("first init");
        init_schema(&conn).expect("second init");
        // No assertion needed — we're checking that the second call
        // doesn't error on existing tables / virtual tables.
    }

    #[test]
    fn version_mismatch_drops_fts_table() {
        let conn = open_in_memory();
        init_schema(&conn).expect("first init");
        // Simulate a pre-existing index from a future/past schema
        // version by writing a different user_version.
        conn.pragma_update(None, "user_version", 999_i64).unwrap();
        let outcome = init_schema(&conn).expect("second init at mismatched version");
        assert!(outcome.fts_rebuilt, "version mismatch must trigger rebuild");
        let version: i64 = conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, INDEX_SCHEMA_VERSION);
    }

    #[test]
    fn fts5_virtual_table_actually_exists() {
        let conn = open_in_memory();
        init_schema(&conn).expect("init");
        // `unicode61` tokenizer needs FTS5 compiled in — this query
        // would fail at parse time on a sqlite without FTS5.
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master \
                 WHERE type = 'table' AND name = 'entities_fts'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn cascade_delete_removes_dependent_rows() {
        let conn = open_in_memory();
        init_schema(&conn).expect("init");
        // Foreign keys are off by default in sqlite — turn them on so
        // the ON DELETE CASCADE actually triggers. Caller code (the
        // index module) does the same.
        conn.execute("PRAGMA foreign_keys = ON", []).unwrap();
        conn.execute(
            "INSERT INTO entities(file_id, type, name, path, mtime_ns, body, frontmatter) \
             VALUES ('x', 'note', 'X', '/x.md', 0, '', '{}')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO entity_tags(file_id, tag) VALUES ('x', 'foo')",
            [],
        )
        .unwrap();
        conn.execute("DELETE FROM entities WHERE file_id = 'x'", [])
            .unwrap();
        let tag_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM entity_tags", [], |r| r.get(0))
            .unwrap();
        assert_eq!(tag_count, 0);
    }
}
