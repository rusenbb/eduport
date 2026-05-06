import sqlite3
from pathlib import Path

from eduport.index.schema import init_schema
from eduport.index.writer import (
    clear_parse_error,
    delete_entity,
    record_parse_error,
    upsert_entity,
)
from eduport.models import University


def _conn(tmp_path: Path) -> sqlite3.Connection:
    conn = sqlite3.connect(tmp_path / "x.db")
    init_schema(conn)
    return conn


def _make_uni() -> University:
    return University.model_validate({
        "tags": ["eduport-type/university", "switzerland"],
        "name": "ETH",
        "country": "Switzerland",
    })


def test_upsert_inserts(tmp_path):
    conn = _conn(tmp_path)
    upsert_entity(
        conn,
        file_id="eth-K9p3",
        path=Path("/data/eth-K9p3.md"),
        mtime_ns=12345,
        entity=_make_uni(),
        body="Body",
    )
    rows = conn.execute("SELECT file_id, type, name FROM entities").fetchall()
    assert rows == [("eth-K9p3", "university", "ETH")]

    tags = {row[0] for row in conn.execute(
        "SELECT tag FROM entity_tags WHERE file_id='eth-K9p3'"
    )}
    assert tags == {"eduport-type/university", "switzerland"}

    fts = conn.execute(
        "SELECT body FROM entities_fts WHERE rowid = (SELECT rowid FROM entities WHERE file_id='eth-K9p3')"
    ).fetchone()
    assert fts == ("Body",)


def test_upsert_updates_on_conflict(tmp_path):
    conn = _conn(tmp_path)
    upsert_entity(conn, "eth-K9p3", Path("/x.md"), 1, _make_uni(), "old body")
    new_uni = University.model_validate({
        "tags": ["eduport-type/university"],
        "name": "ETH (renamed)",
        "country": "CH",
    })
    upsert_entity(conn, "eth-K9p3", Path("/x.md"), 2, new_uni, "new body")

    name = conn.execute("SELECT name FROM entities").fetchone()[0]
    assert name == "ETH (renamed)"
    body = conn.execute(
        "SELECT body FROM entities_fts WHERE rowid = (SELECT rowid FROM entities WHERE file_id='eth-K9p3')"
    ).fetchone()[0]
    assert body == "new body"
    tags = {row[0] for row in conn.execute("SELECT tag FROM entity_tags")}
    assert tags == {"eduport-type/university"}


def test_delete_clears_everything(tmp_path):
    conn = _conn(tmp_path)
    upsert_entity(conn, "eth-K9p3", Path("/x.md"), 1, _make_uni(), "body")
    delete_entity(conn, "eth-K9p3")
    assert conn.execute("SELECT COUNT(*) FROM entities").fetchone()[0] == 0
    assert conn.execute("SELECT COUNT(*) FROM entity_tags").fetchone()[0] == 0
    assert conn.execute("SELECT COUNT(*) FROM entities_fts").fetchone()[0] == 0


def test_record_and_clear_parse_error(tmp_path):
    conn = _conn(tmp_path)
    record_parse_error(conn, "/data/bad-X1x1.md", "bad yaml")
    rows = conn.execute("SELECT path, message FROM parse_errors").fetchall()
    assert rows == [("/data/bad-X1x1.md", "bad yaml")]

    clear_parse_error(conn, "/data/bad-X1x1.md")
    assert conn.execute("SELECT COUNT(*) FROM parse_errors").fetchone()[0] == 0


def test_record_overwrites(tmp_path):
    conn = _conn(tmp_path)
    record_parse_error(conn, "/data/bad-X.md", "first")
    record_parse_error(conn, "/data/bad-X.md", "second")
    rows = conn.execute("SELECT message FROM parse_errors").fetchall()
    assert rows == [("second",)]
