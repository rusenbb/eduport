import sqlite3
from pathlib import Path

import pytest

from eduport.index.reader import backlinks, list_entities, search_fts
from eduport.index.schema import init_schema
from eduport.index.writer import upsert_entity
from eduport.models import Person, Program, University


@pytest.fixture
def conn(tmp_path) -> sqlite3.Connection:
    c = sqlite3.connect(tmp_path / "x.db")
    init_schema(c)

    eth = University.model_validate({
        "tags": ["eduport-type/university", "switzerland"],
        "name": "ETH", "country": "Switzerland",
    })
    msc = Program.model_validate({
        "tags": ["eduport-type/program", "ai"],
        "name": "MSc CS", "level": "masters",
        "university": "[[eth-K9p3]]",
        "people": ["[[jane-A4f2]]"],
    })
    jane = Person.model_validate({
        "tags": ["eduport-type/person", "ai"],
        "name": "Jane Doe", "role": "Professor",
    })
    upsert_entity(c, "eth-K9p3", Path("/x/eth.md"), 1, eth, "Body of ETH")
    upsert_entity(c, "msc-Q7w8", Path("/x/msc.md"), 1, msc, "Body of MSc CS")
    upsert_entity(c, "jane-A4f2", Path("/x/jane.md"), 1, jane, "Body about machine learning")
    return c


def test_list_by_type(conn):
    universities = list_entities(conn, type="university")
    assert [r["file_id"] for r in universities] == ["eth-K9p3"]


def test_list_filter_by_tag(conn):
    ai_only = list_entities(conn, tags=["ai"])
    ids = sorted(r["file_id"] for r in ai_only)
    assert ids == ["jane-A4f2", "msc-Q7w8"]


def test_list_filter_by_multiple_tags_and(conn):
    rows = list_entities(conn, tags=["ai", "switzerland"])
    assert rows == []  # AND semantics — no entity has both


def test_search_body(conn):
    hits = search_fts(conn, "machine learning")
    assert [h["file_id"] for h in hits] == ["jane-A4f2"]


def test_backlinks(conn):
    incoming = backlinks(conn, "jane-A4f2")
    assert [b["src_file_id"] for b in incoming] == ["msc-Q7w8"]
    assert incoming[0]["field"] == "people"
