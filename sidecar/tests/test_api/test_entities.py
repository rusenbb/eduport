from pathlib import Path

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
