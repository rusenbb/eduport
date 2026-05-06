import sqlite3

from eduport.index.schema import init_schema


def test_creates_all_tables(tmp_path):
    conn = sqlite3.connect(tmp_path / "test.db")
    init_schema(conn)

    tables = {row[0] for row in conn.execute(
        "SELECT name FROM sqlite_master WHERE type IN ('table','view')"
    )}
    expected = {
        "entities", "entity_tags", "entity_links",
        "checkboxes", "parse_errors", "entities_fts",
    }
    assert expected <= tables


def test_idempotent(tmp_path):
    conn = sqlite3.connect(tmp_path / "test.db")
    init_schema(conn)
    init_schema(conn)


def test_fts_supports_match(tmp_path):
    conn = sqlite3.connect(tmp_path / "test.db")
    init_schema(conn)
    conn.execute(
        "INSERT INTO entities_fts(rowid, body) VALUES (1, 'the quick brown fox')"
    )
    rows = conn.execute(
        "SELECT rowid FROM entities_fts WHERE entities_fts MATCH 'fox'"
    ).fetchall()
    assert rows == [(1,)]
