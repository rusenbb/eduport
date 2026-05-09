"""HTTP-level tests for /api/properties (counts + filter/sort)."""

from __future__ import annotations

import pytest

from eduport.index.writer import upsert_entity
from eduport.models import University


@pytest.fixture
def populated(client, conn, settings):
    """Add a tier + rank schema and seed three universities."""
    client.post(
        "/api/schema/types/university/properties",
        json={
            "type": "single-select",
            "key": "tier",
            "name": "Tier",
            "options": [
                {"value": "reach", "label": "Reach", "color": "red"},
                {"value": "safety", "label": "Safety", "color": "green"},
            ],
        },
    )
    client.post(
        "/api/schema/types/university/properties",
        json={"type": "number", "key": "rank", "name": "Rank"},
    )

    schema = client.app.state.eduport.schema_store.current()
    for i, (name, tier, rank) in enumerate(
        [("Alpha", "reach", 5.0), ("Beta", "safety", 1.0), ("Gamma", "reach", 3.0)]
    ):
        ent = University.model_validate({
            "tags": ["eduport-type/university"],
            "name": name, "country": "X",
            "tier": tier, "rank": rank,
        })
        upsert_entity(
            conn,
            f"u-{i:04d}",
            settings.data_folder / f"u-{i:04d}.md",
            i + 1,
            ent,
            "",
            schema=schema,
        )
    return client


def test_counts_for_tier(populated):
    r = populated.get("/api/properties/counts/university/tier")
    assert r.status_code == 200
    body = r.json()
    assert body["entity_type"] == "university"
    assert body["key"] == "tier"
    values = {row["value"]: row["count"] for row in body["values"]}
    assert values == {"reach": 2, "safety": 1}


def test_filter_by_tier(populated):
    r = populated.get("/api/properties/filter/university?text=tier:reach")
    assert r.status_code == 200
    names = sorted(row["name"] for row in r.json())
    assert names == ["Alpha", "Gamma"]


def test_filter_by_num_range_open_upper(populated):
    r = populated.get("/api/properties/filter/university?num=rank:..3.5")
    names = sorted(row["name"] for row in r.json())
    assert names == ["Beta", "Gamma"]


def test_sort_by_rank_desc(populated):
    r = populated.get("/api/properties/filter/university?sort=rank&sort_dir=desc")
    names = [row["name"] for row in r.json()]
    assert names == ["Alpha", "Gamma", "Beta"]


def test_filter_then_sort(populated):
    r = populated.get(
        "/api/properties/filter/university?text=tier:reach&sort=rank&sort_dir=asc"
    )
    names = [row["name"] for row in r.json()]
    assert names == ["Gamma", "Alpha"]


def test_unknown_type_returns_422(client):
    r = client.get("/api/properties/counts/wizard/tier")
    assert r.status_code == 422
