
import pytest

from eduport.index.writer import upsert_entity
from eduport.models import University


@pytest.fixture
def seeded_client(client, conn, settings):
    eth = University.model_validate({
        "tags": ["eduport-type/university", "switzerland"],
        "name": "ETH", "country": "Switzerland",
    })
    upsert_entity(conn, "eth-K9p3", settings.data_folder / "eth-K9p3.md", 1, eth, "Body")
    return client


def test_list_universities(seeded_client):
    response = seeded_client.get("/entities/university")
    assert response.status_code == 200
    payload = response.json()
    assert len(payload) == 1
    assert payload[0]["file_id"] == "eth-K9p3"


def test_list_with_tag_filter(seeded_client):
    response = seeded_client.get("/entities/university?tag=switzerland")
    assert response.status_code == 200
    assert len(response.json()) == 1


def test_list_unknown_type_returns_400(client):
    response = client.get("/entities/imaginary")
    assert response.status_code == 400


def test_get_one(seeded_client):
    response = seeded_client.get("/entities/university/eth-K9p3")
    assert response.status_code == 200
    body = response.json()
    assert body["entity"]["name"] == "ETH"
    assert "body" in body
    assert "backlinks" in body
    # Schema is empty — the entity has no custom keys, so no warnings.
    assert body["value_warnings"] == []


def test_get_one_includes_value_warnings_for_orphaned_keys(client, conn, settings):
    """When the entity carries a key not in the schema, it should surface
    as an `orphaned` warning in the GET response."""
    eth = University.model_validate({
        "tags": ["eduport-type/university"],
        "name": "ETH", "country": "Switzerland",
        "tier": "reach",  # not in the (empty) schema → orphaned
    })
    upsert_entity(conn, "eth-K9p3", settings.data_folder / "eth-K9p3.md", 1, eth, "Body")

    response = client.get("/entities/university/eth-K9p3")
    assert response.status_code == 200
    warnings = response.json()["value_warnings"]
    assert len(warnings) == 1
    assert warnings[0]["key"] == "tier"
    assert warnings[0]["kind"] == "orphaned"


def test_get_one_no_warnings_after_property_declared(client, conn, settings):
    """After declaring `tier` as a single-select with `reach`, the same
    entity payload should yield zero warnings."""
    eth = University.model_validate({
        "tags": ["eduport-type/university"],
        "name": "ETH", "country": "Switzerland",
        "tier": "reach",
    })
    upsert_entity(conn, "eth-K9p3", settings.data_folder / "eth-K9p3.md", 1, eth, "Body")

    client.post(
        "/api/schema/types/university/properties",
        json={
            "type": "single-select",
            "key": "tier",
            "name": "Tier",
            "options": [{"value": "reach", "label": "Reach", "color": "red"}],
        },
    )

    response = client.get("/entities/university/eth-K9p3")
    assert response.status_code == 200
    assert response.json()["value_warnings"] == []


def test_get_one_out_of_options_warning(client, conn, settings):
    """Tier value not in the option list → `out_of_options` warning."""
    eth = University.model_validate({
        "tags": ["eduport-type/university"],
        "name": "ETH", "country": "Switzerland",
        "tier": "platinum",  # not in our option list
    })
    upsert_entity(conn, "eth-K9p3", settings.data_folder / "eth-K9p3.md", 1, eth, "Body")

    client.post(
        "/api/schema/types/university/properties",
        json={
            "type": "single-select",
            "key": "tier",
            "name": "Tier",
            "options": [{"value": "reach", "label": "Reach", "color": "red"}],
        },
    )

    response = client.get("/entities/university/eth-K9p3")
    assert response.status_code == 200
    warnings = response.json()["value_warnings"]
    assert warnings[0]["kind"] == "out_of_options"
    assert warnings[0]["value"] == "platinum"


def test_resolve_entity(seeded_client):
    response = seeded_client.get("/entities/resolve/eth-K9p3")
    assert response.status_code == 200
    assert response.json()["type"] == "university"


def test_get_missing_returns_404(client):
    response = client.get("/entities/university/ghost-Z9z9")
    assert response.status_code == 404


def test_create_entity(client, settings):
    payload = {
        "tags": ["eduport-type/university", "ai"],
        "name": "MIT",
        "country": "USA",
    }
    response = client.post(
        "/entities/university",
        json={"frontmatter": payload, "body": "Notes about MIT."},
    )
    assert response.status_code == 201
    file_id = response.json()["file_id"]
    assert file_id.startswith("mit-")
    assert (settings.data_folder / f"{file_id}.md").exists()


def test_update_entity(seeded_client):
    response = seeded_client.patch(
        "/entities/university/eth-K9p3",
        json={
            "frontmatter": {
                "tags": ["eduport-type/university"],
                "name": "ETH (renamed)",
                "country": "Switzerland",
            },
            "body": "Updated notes.",
        },
    )
    assert response.status_code == 200
    after = seeded_client.get("/entities/university/eth-K9p3").json()
    assert after["entity"]["name"] == "ETH (renamed)"
    assert after["body"] == "Updated notes."


def test_delete_moves_to_trash(seeded_client, settings):
    # We need the file to actually exist on disk for trash to work.
    (settings.data_folder / "eth-K9p3.md").write_text("placeholder", encoding="utf-8")

    response = seeded_client.delete("/entities/university/eth-K9p3")
    assert response.status_code == 204
    assert seeded_client.get("/entities/university/eth-K9p3").status_code == 404
    assert not (settings.data_folder / "eth-K9p3.md").exists()
    trashed = list((settings.data_folder / ".eduport-trash").glob("eth-K9p3*"))
    assert trashed
