def test_full_flow_create_read_search_delete(client):
    # Create a University
    create = client.post(
        "/entities/university",
        json={
            "frontmatter": {
                "tags": ["eduport-type/university", "switzerland"],
                "name": "ETH Zurich",
                "country": "Switzerland",
            },
            "body": "Strong AI track here",
        },
    )
    assert create.status_code == 201
    file_id = create.json()["file_id"]

    # List
    listed = client.get("/entities/university").json()
    assert any(r["file_id"] == file_id for r in listed)

    # Get with backlinks
    one = client.get(f"/entities/university/{file_id}").json()
    assert one["entity"]["name"] == "ETH Zurich"
    assert one["body"] == "Strong AI track here"
    assert one["backlinks"] == []

    # Search
    hits = client.get("/search?q=track").json()
    assert any(h["file_id"] == file_id for h in hits)

    # Update
    upd = client.patch(
        f"/entities/university/{file_id}",
        json={
            "frontmatter": {
                "tags": ["eduport-type/university"],
                "name": "ETH Zurich (renamed)",
                "country": "Switzerland",
            },
            "body": "Updated body without that keyword",
        },
    )
    assert upd.status_code == 200
    after = client.get(f"/entities/university/{file_id}").json()
    assert after["entity"]["name"] == "ETH Zurich (renamed)"

    # Search must reflect new body
    no_track = [h for h in client.get("/search?q=track").json() if h["file_id"] == file_id]
    assert no_track == []

    # Delete
    deleted = client.delete(f"/entities/university/{file_id}")
    assert deleted.status_code == 204
    assert client.get(f"/entities/university/{file_id}").status_code == 404
