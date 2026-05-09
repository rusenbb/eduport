"""Tests for the FTS5 ``custom_text`` column and schema-version migration."""

from __future__ import annotations

import sqlite3
from pathlib import Path

import pytest

from eduport.index.schema import INDEX_SCHEMA_VERSION, init_schema
from eduport.index.writer import upsert_entity
from eduport.models import (
    EntityType,
    TextProperty,
    University,
    UrlProperty,
    empty_schema,
)
from eduport.models.schema import EntitySchema


def _university(**extras) -> University:
    return University.model_validate(
        {
            "name": "Test U",
            "country": "X",
            "tags": ["eduport-type/university"],
            **extras,
        }
    )


def _schema_with(entity_type: EntityType, props):
    schema = empty_schema()
    schema.types[entity_type] = EntitySchema(properties=props)
    return schema


@pytest.fixture
def db(tmp_path: Path) -> sqlite3.Connection:
    conn = sqlite3.connect(":memory:")
    init_schema(conn)
    return conn


class TestSchemaVersion:
    def test_fresh_db_sets_version(self, db: sqlite3.Connection) -> None:
        version = db.execute("PRAGMA user_version").fetchone()[0]
        assert version == INDEX_SCHEMA_VERSION

    def test_fresh_db_reports_no_migration(self) -> None:
        conn = sqlite3.connect(":memory:")
        migrated = init_schema(conn)
        assert migrated is False

    def test_old_version_triggers_migration(self) -> None:
        conn = sqlite3.connect(":memory:")
        # Simulate an old database: create the v1 schema (FTS5 without custom_text)
        # and pin user_version to 1.
        conn.executescript(
            """
            CREATE VIRTUAL TABLE IF NOT EXISTS entities_fts USING fts5(
                body, name, tags,
                tokenize="unicode61 remove_diacritics 2"
            );
            CREATE TABLE IF NOT EXISTS entities (
                file_id TEXT PRIMARY KEY,
                type TEXT, name TEXT, path TEXT,
                mtime_ns INTEGER, body TEXT, frontmatter TEXT
            );
            PRAGMA user_version = 1;
            """
        )
        migrated = init_schema(conn)
        assert migrated is True
        # FTS5 should now have custom_text:
        cols = [
            row[1]
            for row in conn.execute("PRAGMA table_info(entities_fts)").fetchall()
        ]
        assert "custom_text" in cols
        # Version stamped:
        assert conn.execute("PRAGMA user_version").fetchone()[0] == INDEX_SCHEMA_VERSION

    def test_current_version_no_migration(self) -> None:
        conn = sqlite3.connect(":memory:")
        init_schema(conn)
        migrated = init_schema(conn)  # second call
        assert migrated is False


class TestCustomTextPopulation:
    def test_text_property_value_in_fts5(self, db: sqlite3.Connection, tmp_path: Path) -> None:
        schema = _schema_with(
            EntityType.university,
            [TextProperty(type="text", key="motto", name="Motto")],
        )
        ent = _university(motto="Veritas in unitate")
        upsert_entity(
            db, "u-AAAA", tmp_path / "u-AAAA.md", 1, ent, "irrelevant body",
            schema=schema,
        )
        rows = db.execute(
            "SELECT custom_text FROM entities_fts WHERE custom_text MATCH ?",
            ("veritas",),
        ).fetchall()
        assert len(rows) == 1
        assert "Veritas" in rows[0][0]

    def test_url_property_value_in_fts5(self, db: sqlite3.Connection, tmp_path: Path) -> None:
        schema = _schema_with(
            EntityType.university,
            [UrlProperty(type="url", key="page", name="Page")],
        )
        ent = _university(page="https://example.com/programs")
        upsert_entity(
            db, "u-AAAA", tmp_path / "u-AAAA.md", 1, ent, "",
            schema=schema,
        )
        rows = db.execute(
            "SELECT custom_text FROM entities_fts WHERE custom_text MATCH ?",
            ("programs",),
        ).fetchall()
        assert len(rows) == 1

    def test_non_text_properties_excluded(self, db: sqlite3.Connection, tmp_path: Path) -> None:
        from eduport.models import NumberProperty

        schema = _schema_with(
            EntityType.university,
            [NumberProperty(type="number", key="rank", name="Rank")],
        )
        ent = _university(rank=42)
        upsert_entity(
            db, "u-AAAA", tmp_path / "u-AAAA.md", 1, ent, "",
            schema=schema,
        )
        custom = db.execute("SELECT custom_text FROM entities_fts").fetchone()[0]
        assert custom == ""

    def test_full_text_search_finds_custom_value(
        self, db: sqlite3.Connection, tmp_path: Path
    ) -> None:
        """Cross-column search: matching on `custom_text` returns the entity."""
        from eduport.index.reader import search_fts

        schema = _schema_with(
            EntityType.university,
            [TextProperty(type="text", key="motto", name="Motto")],
        )
        ent = _university(motto="LiberateMente")
        upsert_entity(
            db, "u-AAAA", tmp_path / "u-AAAA.md", 1, ent, "Body has nothing matching.",
            schema=schema,
        )
        results = search_fts(db, "LiberateMente")
        assert len(results) == 1
        assert results[0]["file_id"] == "u-AAAA"
