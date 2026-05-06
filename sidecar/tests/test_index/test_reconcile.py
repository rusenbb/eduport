import sqlite3
from pathlib import Path

import pytest

from eduport.index.reconcile import reconcile
from eduport.index.schema import init_schema


def _write(folder: Path, name: str, content: str) -> Path:
    p = folder / name
    p.write_text(content, encoding="utf-8")
    return p


@pytest.fixture
def conn_and_folder(tmp_path):
    folder = tmp_path / "data"
    folder.mkdir()
    conn = sqlite3.connect(tmp_path / "x.db")
    init_schema(conn)
    return conn, folder


def test_reconcile_inserts_new_files(conn_and_folder):
    conn, folder = conn_and_folder
    _write(folder, "eth-K9p3.md", """---
tags: [eduport-type/university]
name: ETH
country: CH
---
""")
    summary = reconcile(conn, folder)
    assert summary.added == 1
    assert summary.errors == 0
    assert conn.execute("SELECT COUNT(*) FROM entities").fetchone()[0] == 1


def test_reconcile_records_parse_errors(conn_and_folder):
    conn, folder = conn_and_folder
    _write(folder, "bad-X1x1.md", "no frontmatter at all")
    summary = reconcile(conn, folder)
    assert summary.errors == 1
    assert conn.execute("SELECT COUNT(*) FROM parse_errors").fetchone()[0] == 1


def test_reconcile_removes_missing_files(conn_and_folder):
    conn, folder = conn_and_folder
    f = _write(folder, "eth-K9p3.md", """---
tags: [eduport-type/university]
name: ETH
country: CH
---
""")
    reconcile(conn, folder)
    f.unlink()
    summary = reconcile(conn, folder)
    assert summary.removed == 1
    assert conn.execute("SELECT COUNT(*) FROM entities").fetchone()[0] == 0
