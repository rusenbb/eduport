def test_status_reports_counts(client):
    response = client.get("/status")

    assert response.status_code == 200
    assert response.json()["status"] == "ok"
    assert "parse_errors" in response.json()
    assert "entities" in response.json()


def test_counts_and_tags(client):
    created = client.post(
        "/entities/note",
        json={
            "frontmatter": {"tags": ["eduport-type/note", "sample"], "name": "Tagged"},
            "body": "Body",
        },
    )
    assert created.status_code == 201

    counts = client.get("/counts")
    assert counts.status_code == 200
    assert counts.json()["note"] == 1

    tags = client.get("/tags")
    assert tags.status_code == 200
    assert {"tag": "sample", "count": 1} in tags.json()


def test_trash_restore_round_trip(client, settings):
    create = client.post(
        "/entities/note",
        json={
            "frontmatter": {
                "tags": ["eduport-type/note", "sample"],
                "name": "Restore Me",
            },
            "body": "Body",
        },
    )
    assert create.status_code == 201
    file_id = create.json()["file_id"]

    deleted = client.delete(f"/entities/note/{file_id}")
    assert deleted.status_code == 204

    listed = client.get("/trash")
    assert listed.status_code == 200
    item = next(i for i in listed.json() if i["name"] == f"{file_id}.md")

    restored = client.post("/trash/restore", json={"name": item["name"]})
    assert restored.status_code == 200
    assert restored.json()["file_id"] == file_id
    assert (settings.data_folder / f"{file_id}.md").exists()
    assert client.get(f"/entities/note/{file_id}").status_code == 200
