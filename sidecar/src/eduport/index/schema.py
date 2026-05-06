import sqlite3

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

CREATE VIRTUAL TABLE IF NOT EXISTS entities_fts USING fts5(
    body,
    name,
    tags,
    tokenize="unicode61 remove_diacritics 2"
);
"""


def init_schema(conn: sqlite3.Connection) -> None:
    conn.executescript(_DDL)
    conn.commit()
