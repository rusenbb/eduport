"""Tests for the `properties` SQLite table — coercion, upsert, and queries."""

from __future__ import annotations

import sqlite3
from pathlib import Path

import pytest

from eduport.index.reader import (
    filter_entities_by_properties,
    property_value_counts,
)
from eduport.index.writer import _coerce_property_columns, upsert_entity
from eduport.models import (
    EntityType,
    SelectOption,
    SingleSelectProperty,
    University,
    empty_schema,
)
from eduport.models.schema import EntitySchema


def _university(**extras) -> University:
    return University.model_validate(
        {
            "name": extras.pop("name", "Test U"),
            "country": extras.pop("country", "X"),
            "tags": ["eduport-type/university"],
            **extras,
        }
    )


def _schema_with(entity_type: EntityType, props):
    schema = empty_schema()
    schema.types[entity_type] = EntitySchema(properties=props)
    return schema


class TestCoercion:
    def test_text(self):
        text, num, iso, multi = _coerce_property_columns("text", "hello")
        assert (text, num, iso, multi) == ("hello", None, None, None)

    def test_text_wrong_type_all_null(self):
        assert _coerce_property_columns("text", 42) == (None, None, None, None)

    def test_number_int(self):
        assert _coerce_property_columns("number", 3) == (None, 3.0, None, None)

    def test_number_float(self):
        assert _coerce_property_columns("number", 3.5) == (None, 3.5, None, None)

    def test_number_rejects_bool(self):
        assert _coerce_property_columns("number", True) == (None, None, None, None)

    def test_checkbox_true(self):
        assert _coerce_property_columns("checkbox", True) == (None, 1.0, None, None)

    def test_checkbox_false(self):
        assert _coerce_property_columns("checkbox", False) == (None, 0.0, None, None)

    def test_date_iso(self):
        assert _coerce_property_columns("date", "2026-05-09") == (
            None, None, "2026-05-09", None,
        )

    def test_date_invalid(self):
        assert _coerce_property_columns("date", "yesterday") == (None, None, None, None)

    def test_single_select(self):
        text, *_ = _coerce_property_columns("single-select", "reach")
        assert text == "reach"

    def test_multi_select(self):
        _, _, _, multi = _coerce_property_columns("multi-select", ["a", "b"])
        assert multi == '["a", "b"]'

    def test_url(self):
        assert _coerce_property_columns("url", "https://x.com") == (
            "https://x.com", None, None, None,
        )

    def test_relation(self):
        text, *_ = _coerce_property_columns("relation", "[[some-A1B2]]")
        assert text == "[[some-A1B2]]"


class TestUpsertProperties:
    def _conn(self) -> sqlite3.Connection:
        from eduport.index.schema import init_schema

        c = sqlite3.connect(":memory:")
        init_schema(c)
        return c

    def test_no_schema_skips_properties_table(self, tmp_path: Path):
        conn = self._conn()
        ent = _university(rogue="x")
        upsert_entity(
            conn,
            file_id="u-AAAA",
            path=tmp_path / "u-AAAA.md",
            mtime_ns=1,
            entity=ent,
            body="",
            schema=None,
        )
        rows = conn.execute("SELECT * FROM properties").fetchall()
        assert rows == []

    def test_writes_only_declared_keys(self, tmp_path: Path):
        conn = self._conn()
        schema = _schema_with(
            EntityType.university,
            [
                SingleSelectProperty(
                    type="single-select",
                    key="tier",
                    name="Tier",
                    options=[
                        SelectOption(value="reach", label="Reach", color="red"),
                    ],
                )
            ],
        )
        ent = _university(tier="reach", orphan="ignored")
        upsert_entity(
            conn,
            file_id="u-AAAA",
            path=tmp_path / "u-AAAA.md",
            mtime_ns=1,
            entity=ent,
            body="",
            schema=schema,
        )
        rows = conn.execute(
            "SELECT key, type, value_text FROM properties"
        ).fetchall()
        assert rows == [("tier", "single-select", "reach")]  # orphan absent

    def test_skips_type_mismatched_values(self, tmp_path: Path):
        from eduport.models.schema import NumberProperty

        conn = self._conn()
        schema = _schema_with(
            EntityType.university,
            [NumberProperty(type="number", key="n", name="N")],
        )
        ent = _university(n="not a number")
        upsert_entity(
            conn,
            file_id="u-AAAA",
            path=tmp_path / "u-AAAA.md",
            mtime_ns=1,
            entity=ent,
            body="",
            schema=schema,
        )
        assert conn.execute("SELECT COUNT(*) FROM properties").fetchone()[0] == 0

    def test_replaces_existing_rows_on_re_upsert(self, tmp_path: Path):
        conn = self._conn()
        schema = _schema_with(
            EntityType.university,
            [
                SingleSelectProperty(
                    type="single-select",
                    key="tier",
                    name="Tier",
                    options=[
                        SelectOption(value="reach", label="Reach"),
                        SelectOption(value="safety", label="Safety"),
                    ],
                )
            ],
        )
        ent1 = _university(tier="reach")
        upsert_entity(
            conn, "u-AAAA", tmp_path / "u-AAAA.md", 1, ent1, "", schema=schema
        )
        ent2 = _university(tier="safety")
        upsert_entity(
            conn, "u-AAAA", tmp_path / "u-AAAA.md", 2, ent2, "", schema=schema
        )
        rows = conn.execute("SELECT value_text FROM properties").fetchall()
        assert rows == [("safety",)]


class TestPropertyValueCounts:
    @pytest.fixture
    def populated(self, tmp_path: Path):
        from eduport.index.schema import init_schema

        conn = sqlite3.connect(":memory:")
        init_schema(conn)
        schema = _schema_with(
            EntityType.university,
            [
                SingleSelectProperty(
                    type="single-select",
                    key="tier",
                    name="Tier",
                    options=[
                        SelectOption(value="reach", label="Reach"),
                        SelectOption(value="safety", label="Safety"),
                    ],
                )
            ],
        )
        for i, tier in enumerate(["reach", "reach", "safety", None]):
            extras = {"tier": tier} if tier else {}
            ent = _university(name=f"U{i}", **extras)
            upsert_entity(
                conn, f"u-{i:04d}", tmp_path / f"u-{i:04d}.md", i + 1, ent, "",
                schema=schema,
            )
        return conn

    def test_counts(self, populated):
        counts = property_value_counts(populated, "university", "tier")
        # 2 reach > 1 safety
        assert counts == [
            {"type": "single-select", "value": "reach", "count": 2},
            {"type": "single-select", "value": "safety", "count": 1},
        ]


class TestFilterAndSort:
    @pytest.fixture
    def populated(self, tmp_path: Path):
        from eduport.index.schema import init_schema
        from eduport.models.schema import NumberProperty

        conn = sqlite3.connect(":memory:")
        init_schema(conn)
        schema = _schema_with(
            EntityType.university,
            [
                SingleSelectProperty(
                    type="single-select",
                    key="tier",
                    name="Tier",
                    options=[
                        SelectOption(value="reach", label="Reach"),
                        SelectOption(value="safety", label="Safety"),
                    ],
                ),
                NumberProperty(type="number", key="rank", name="Rank"),
            ],
        )
        for i, (name, tier, rank) in enumerate(
            [
                ("Alpha", "reach", 5.0),
                ("Beta", "safety", 1.0),
                ("Gamma", "reach", 3.0),
                ("Delta", "safety", 9.0),
            ]
        ):
            ent = _university(name=name, tier=tier, rank=rank)
            upsert_entity(
                conn, f"u-{i:04d}", tmp_path / f"u-{i:04d}.md", i + 1, ent, "",
                schema=schema,
            )
        return conn

    def test_filter_by_text(self, populated):
        rows = filter_entities_by_properties(
            populated, "university", text_filters={"tier": "reach"}
        )
        names = [r["name"] for r in rows]
        assert names == ["Alpha", "Gamma"]

    def test_filter_by_num_range(self, populated):
        rows = filter_entities_by_properties(
            populated,
            "university",
            num_range_filters={"rank": (None, 5.0)},
        )
        names = sorted(r["name"] for r in rows)
        assert names == ["Alpha", "Beta", "Gamma"]

    def test_sort_by_num_asc(self, populated):
        rows = filter_entities_by_properties(
            populated, "university", sort_key="rank", sort_dir="asc"
        )
        names = [r["name"] for r in rows]
        assert names == ["Beta", "Gamma", "Alpha", "Delta"]

    def test_sort_by_num_desc(self, populated):
        rows = filter_entities_by_properties(
            populated, "university", sort_key="rank", sort_dir="desc"
        )
        names = [r["name"] for r in rows]
        assert names == ["Delta", "Alpha", "Gamma", "Beta"]

    def test_filter_then_sort(self, populated):
        rows = filter_entities_by_properties(
            populated,
            "university",
            text_filters={"tier": "reach"},
            sort_key="rank",
            sort_dir="asc",
        )
        names = [r["name"] for r in rows]
        assert names == ["Gamma", "Alpha"]
