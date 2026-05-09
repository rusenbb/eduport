import sqlite3

INDEX_SCHEMA_VERSION = 2  # bumped: FTS5 grew a `custom_text` column

_DDL = """
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

CREATE TABLE IF NOT EXISTS entity_links (
    src_file_id TEXT NOT NULL REFERENCES entities(file_id) ON DELETE CASCADE,
    field       TEXT NOT NULL,
    target      TEXT NOT NULL,
    resolved    TEXT,
    PRIMARY KEY (src_file_id, field, target)
);
CREATE INDEX IF NOT EXISTS idx_links_resolved ON entity_links(resolved);

CREATE TABLE IF NOT EXISTS checkboxes (
    file_id  TEXT NOT NULL REFERENCES entities(file_id) ON DELETE CASCADE,
    line     INTEGER NOT NULL,
    checked  INTEGER NOT NULL,
    text     TEXT NOT NULL,
    deadline TEXT,
    PRIMARY KEY (file_id, line)
);

CREATE TABLE IF NOT EXISTS parse_errors (
    path        TEXT PRIMARY KEY,
    message     TEXT NOT NULL,
    occurred_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- One row per (entity, custom property key). Filled by the indexer from
-- the entity's `model_extra` keys that match the loaded schema. Built-in
-- fields are NOT mirrored here.
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

-- Full-text search. `custom_text` carries concatenated text/url custom-property
-- values so command-palette search picks them up alongside the body/name/tags.
CREATE VIRTUAL TABLE IF NOT EXISTS entities_fts USING fts5(
    body,
    name,
    tags,
    custom_text,
    tokenize="unicode61 remove_diacritics 2"
);
"""


def init_schema(conn: sqlite3.Connection) -> bool:
    """Apply DDL and any version migrations.

    Returns True if a migration changed the FTS5 schema (so the caller knows
    it should re-index from the entities table). Fresh databases are *not*
    flagged as migrated — they have no entities yet.
    """
    cur = conn.cursor()
    current_version = cur.execute("PRAGMA user_version").fetchone()[0]
    fts5_migrated = False

    if 0 < current_version < INDEX_SCHEMA_VERSION:
        # Old database — drop FTS5 (its column set may differ); the new DDL
        # will recreate it with the current shape. The caller must repopulate.
        cur.execute("DROP TABLE IF EXISTS entities_fts")
        fts5_migrated = True

    conn.executescript(_DDL)
    cur.execute(f"PRAGMA user_version = {INDEX_SCHEMA_VERSION}")
    conn.commit()
    return fts5_migrated
