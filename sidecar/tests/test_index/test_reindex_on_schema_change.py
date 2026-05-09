"""End-to-end test that schema-API mutations trigger a properties re-index."""

from __future__ import annotations

import pytest

from eduport.index.writer import upsert_entity
from eduport.models import University


@pytest.fixture
def seeded_with_tier(client, conn, settings):
    """Seed a university whose YAML has a `tier` extra key, *before* the
    schema declares `tier`. The properties table should be empty at this
    point — there's no schema to validate against.
    """
    eth = University.model_validate({
        "tags": ["eduport-type/university"],
        "name": "ETH", "country": "Switzerland",
        "tier": "reach",
    })
    upsert_entity(
        conn,
        "eth-K9p3",
        settings.data_folder / "eth-K9p3.md",
        1,
        eth,
        "",
        schema=client.app.state.eduport.schema_store.current(),
    )
    return client


def test_adding_property_indexes_existing_values(seeded_with_tier, conn):
    # No properties row before declaring tier:
    assert conn.execute("SELECT COUNT(*) FROM properties").fetchone()[0] == 0
    # Declare tier:
    seeded_with_tier.post(
        "/api/schema/types/university/properties",
        json={
            "type": "single-select",
            "key": "tier",
            "name": "Tier",
            "options": [{"value": "reach", "label": "Reach", "color": "red"}],
        },
    )
    rows = conn.execute(
        "SELECT key, value_text FROM properties WHERE file_id = 'eth-K9p3'"
    ).fetchall()
    assert rows == [("tier", "reach")]


def test_deleting_property_removes_indexed_values(seeded_with_tier, conn):
    seeded_with_tier.post(
        "/api/schema/types/university/properties",
        json={
            "type": "single-select",
            "key": "tier",
            "name": "Tier",
            "options": [{"value": "reach", "label": "Reach"}],
        },
    )
    assert conn.execute("SELECT COUNT(*) FROM properties").fetchone()[0] == 1

    seeded_with_tier.delete("/api/schema/types/university/properties/tier")
    assert conn.execute("SELECT COUNT(*) FROM properties").fetchone()[0] == 0


def test_template_indexes_values(seeded_with_tier, conn):
    seeded_with_tier.post(
        "/api/schema/templates/tier",
        json={"types": ["university"]},
    )
    rows = conn.execute(
        "SELECT key, value_text FROM properties WHERE file_id = 'eth-K9p3'"
    ).fetchall()
    assert rows == [("tier", "reach")]
