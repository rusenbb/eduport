"""HTTP-level tests for /api/views."""

from __future__ import annotations

from fastapi.testclient import TestClient


class TestRead:
    def test_get_seeded(self, client: TestClient) -> None:
        r = client.get("/api/views")
        assert r.status_code == 200
        body = r.json()
        assert body["version"] == 1
        assert set(body["types"].keys()) == {
            "university", "lab", "person", "program",
            "application", "document", "email", "note",
        }
        assert body["types"]["university"]["views"] == []

    def test_get_one_type(self, client: TestClient) -> None:
        r = client.get("/api/views/types/university")
        assert r.status_code == 200
        assert r.json() == {"entity_type": "university", "views": []}


class TestCreate:
    def test_basic(self, client: TestClient) -> None:
        r = client.post(
            "/api/views/types/university",
            json={"name": "Reach schools", "kind": "board", "group_by_key": "tier"},
        )
        assert r.status_code == 201
        body = r.json()
        assert body["view"]["name"] == "Reach schools"
        assert body["view"]["kind"] == "board"
        assert body["view"]["id"].startswith("reach-schools-")

    def test_invalid_kind_returns_422(self, client: TestClient) -> None:
        r = client.post(
            "/api/views/types/university",
            json={"name": "X", "kind": "calendar"},
        )
        assert r.status_code == 422

    def test_filter_round_trip(self, client: TestClient) -> None:
        r = client.post(
            "/api/views/types/university",
            json={
                "name": "Filtered",
                "filter": {
                    "text": {"tier": "reach"},
                    "num": {"rank": [1.0, 5.0]},
                    "date": {"deadline": [None, "2026-12-31"]},
                },
            },
        )
        assert r.status_code == 201
        v = r.json()["view"]
        assert v["filter"]["text"] == {"tier": "reach"}


class TestUpdate:
    def test_replace(self, client: TestClient) -> None:
        created = client.post(
            "/api/views/types/university", json={"name": "X"}
        ).json()["view"]
        vid = created["id"]
        r = client.put(
            f"/api/views/types/university/{vid}",
            json={"name": "X renamed", "kind": "table", "columns": ["tier"]},
        )
        assert r.status_code == 200
        assert r.json()["view"]["name"] == "X renamed"
        assert r.json()["view"]["columns"] == ["tier"]

    def test_unknown_returns_404(self, client: TestClient) -> None:
        r = client.put(
            "/api/views/types/university/missing-AAAA",
            json={"name": "X", "kind": "list"},
        )
        assert r.status_code == 404


class TestDelete:
    def test_delete(self, client: TestClient) -> None:
        created = client.post(
            "/api/views/types/university", json={"name": "X"}
        ).json()["view"]
        r = client.delete(f"/api/views/types/university/{created['id']}")
        assert r.status_code == 200
        assert r.json()["views"] == []

    def test_unknown_returns_404(self, client: TestClient) -> None:
        r = client.delete("/api/views/types/university/missing-AAAA")
        assert r.status_code == 404


class TestReorder:
    def test_reorder(self, client: TestClient) -> None:
        a = client.post("/api/views/types/university", json={"name": "A"}).json()["view"]
        b = client.post("/api/views/types/university", json={"name": "B"}).json()["view"]
        c = client.post("/api/views/types/university", json={"name": "C"}).json()["view"]
        r = client.post(
            "/api/views/types/university/reorder",
            json={"ordered_ids": [c["id"], a["id"], b["id"]]},
        )
        assert r.status_code == 200
        ids = [v["id"] for v in r.json()["views"]]
        assert ids == [c["id"], a["id"], b["id"]]

    def test_reorder_mismatch(self, client: TestClient) -> None:
        client.post("/api/views/types/university", json={"name": "A"}).json()
        r = client.post(
            "/api/views/types/university/reorder",
            json={"ordered_ids": ["ghost-AAAA"]},
        )
        assert r.status_code == 409
