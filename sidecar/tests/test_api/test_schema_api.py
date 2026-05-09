"""HTTP-level tests for /api/schema."""

from __future__ import annotations

from fastapi.testclient import TestClient


class TestRead:
    def test_get_full_schema_seeded(self, client: TestClient) -> None:
        r = client.get("/api/schema")
        assert r.status_code == 200
        body = r.json()
        assert body["version"] == 1
        assert set(body["types"].keys()) == {
            "university",
            "lab",
            "person",
            "program",
            "application",
            "document",
            "email",
            "note",
        }
        # built-in keys advertised
        assert "country" in body["types"]["university"]["builtin_keys"]
        assert "status" in body["types"]["application"]["builtin_keys"]

    def test_get_one_type(self, client: TestClient) -> None:
        r = client.get("/api/schema/types/university")
        assert r.status_code == 200
        body = r.json()
        assert body["entity_type"] == "university"
        assert body["properties"] == []

    def test_unknown_type_404(self, client: TestClient) -> None:
        r = client.get("/api/schema/types/wizard")
        assert r.status_code == 422  # FastAPI enum-coercion failure


class TestAddProperty:
    def test_add_single_select(self, client: TestClient) -> None:
        r = client.post(
            "/api/schema/types/university/properties",
            json={
                "type": "single-select",
                "key": "tier",
                "name": "Tier",
                "options": [
                    {"value": "reach", "label": "Reach", "color": "red"},
                ],
            },
        )
        assert r.status_code == 201
        props = r.json()["properties"]
        assert props[0]["key"] == "tier"
        assert props[0]["type"] == "single-select"

    def test_collision_with_builtin_returns_409(self, client: TestClient) -> None:
        r = client.post(
            "/api/schema/types/university/properties",
            json={"type": "text", "key": "country", "name": "Country"},
        )
        assert r.status_code == 409
        assert "built-in" in r.json()["detail"]

    def test_collision_with_existing_returns_409(self, client: TestClient) -> None:
        body = {"type": "text", "key": "ranking", "name": "Ranking"}
        r1 = client.post("/api/schema/types/university/properties", json=body)
        assert r1.status_code == 201
        r2 = client.post("/api/schema/types/university/properties", json=body)
        assert r2.status_code == 409
        assert "already exists" in r2.json()["detail"]

    def test_invalid_property_body_returns_422(self, client: TestClient) -> None:
        r = client.post(
            "/api/schema/types/university/properties",
            json={"type": "text", "key": "Bad Key", "name": "X"},
        )
        assert r.status_code == 422


class TestPatchProperty:
    def test_patch_name_and_description(self, client: TestClient) -> None:
        client.post(
            "/api/schema/types/university/properties",
            json={"type": "text", "key": "notes", "name": "Notes"},
        )
        r = client.patch(
            "/api/schema/types/university/properties/notes",
            json={"name": "Internal notes", "description": "private"},
        )
        assert r.status_code == 200
        prop = r.json()["properties"][0]
        assert prop["name"] == "Internal notes"
        assert prop["description"] == "private"

    def test_patch_unknown_property_returns_404(self, client: TestClient) -> None:
        r = client.patch(
            "/api/schema/types/university/properties/missing",
            json={"name": "x"},
        )
        assert r.status_code == 404

    def test_patch_with_unit_on_text_returns_409(self, client: TestClient) -> None:
        client.post(
            "/api/schema/types/university/properties",
            json={"type": "text", "key": "notes", "name": "Notes"},
        )
        r = client.patch(
            "/api/schema/types/university/properties/notes",
            json={"unit": "kg"},
        )
        assert r.status_code == 409

    def test_patch_options_label_color(self, client: TestClient) -> None:
        client.post(
            "/api/schema/types/university/properties",
            json={
                "type": "single-select",
                "key": "tier",
                "name": "Tier",
                "options": [
                    {"value": "reach", "label": "Reach", "color": "red"},
                ],
            },
        )
        r = client.patch(
            "/api/schema/types/university/properties/tier",
            json={
                "options": [
                    {"value": "reach", "label": "Reaches", "color": "orange"},
                ]
            },
        )
        assert r.status_code == 200
        opts = r.json()["properties"][0]["options"]
        assert opts[0]["label"] == "Reaches"
        assert opts[0]["color"] == "orange"

    def test_patch_extra_field_returns_422(self, client: TestClient) -> None:
        client.post(
            "/api/schema/types/university/properties",
            json={"type": "text", "key": "notes", "name": "Notes"},
        )
        r = client.patch(
            "/api/schema/types/university/properties/notes",
            json={"rogue": True},
        )
        assert r.status_code == 422


class TestDeleteProperty:
    def test_delete(self, client: TestClient) -> None:
        client.post(
            "/api/schema/types/university/properties",
            json={"type": "text", "key": "ranking", "name": "Ranking"},
        )
        r = client.delete("/api/schema/types/university/properties/ranking")
        assert r.status_code == 200
        assert r.json()["properties"] == []

    def test_delete_unknown_returns_404(self, client: TestClient) -> None:
        r = client.delete("/api/schema/types/university/properties/missing")
        assert r.status_code == 404


class TestTierTemplate:
    def test_adds_tier_to_listed_types(self, client: TestClient) -> None:
        r = client.post(
            "/api/schema/templates/tier",
            json={"types": ["university", "program"]},
        )
        assert r.status_code == 201
        results = r.json()["results"]
        assert results["university"]["status"] == "added"
        assert results["program"]["status"] == "added"
        # confirm via get
        s = client.get("/api/schema").json()
        uni_keys = [p["key"] for p in s["types"]["university"]["properties"]]
        prog_keys = [p["key"] for p in s["types"]["program"]["properties"]]
        assert "tier" in uni_keys
        assert "tier" in prog_keys

    def test_idempotent_when_already_present(self, client: TestClient) -> None:
        r1 = client.post("/api/schema/templates/tier", json={"types": ["university"]})
        assert r1.status_code == 201
        r2 = client.post("/api/schema/templates/tier", json={"types": ["university"]})
        assert r2.status_code == 201
        results = r2.json()["results"]
        assert results["university"]["status"] == "exists"

    def test_empty_types_rejected(self, client: TestClient) -> None:
        r = client.post("/api/schema/templates/tier", json={"types": []})
        assert r.status_code == 422


class TestPersistence:
    def test_added_property_persists_to_disk(
        self, client: TestClient, settings
    ) -> None:
        client.post(
            "/api/schema/types/university/properties",
            json={"type": "text", "key": "ranking", "name": "Ranking"},
        )
        schema_file = settings.data_folder / ".eduport" / "schema.yaml"
        assert schema_file.exists()
        text = schema_file.read_text(encoding="utf-8")
        assert "ranking" in text


class TestReorderProperties:
    def test_reorder(self, client: TestClient) -> None:
        for k in ["a", "b", "c"]:
            client.post(
                "/api/schema/types/university/properties",
                json={"type": "text", "key": k, "name": k.upper()},
            )
        r = client.post(
            "/api/schema/types/university/reorder",
            json={"ordered_keys": ["c", "a", "b"]},
        )
        assert r.status_code == 200
        keys = [p["key"] for p in r.json()["properties"]]
        assert keys == ["c", "a", "b"]

    def test_mismatch_returns_409(self, client: TestClient) -> None:
        client.post(
            "/api/schema/types/university/properties",
            json={"type": "text", "key": "a", "name": "A"},
        )
        r = client.post(
            "/api/schema/types/university/reorder",
            json={"ordered_keys": ["a", "ghost"]},
        )
        assert r.status_code == 409


class TestPurgeOrphans:
    def test_refuses_if_key_still_declared(self, client: TestClient) -> None:
        client.post(
            "/api/schema/types/university/properties",
            json={"type": "text", "key": "ranking", "name": "Ranking"},
        )
        r = client.post(
            "/api/schema/types/university/properties/ranking/purge_orphans"
        )
        assert r.status_code == 409

    def test_purges_orphan_from_file(self, client: TestClient, settings) -> None:
        from eduport.index.writer import upsert_entity
        from eduport.models import University

        # Create the property so we can write a tier-bearing entity to disk:
        client.post(
            "/api/schema/types/university/properties",
            json={
                "type": "single-select",
                "key": "tier",
                "name": "Tier",
                "options": [{"value": "reach", "label": "Reach"}],
            },
        )
        ent = University.model_validate({
            "tags": ["eduport-type/university"],
            "name": "ETH", "country": "Switzerland",
            "tier": "reach",
        })
        # Write to disk via the file_store the way the production CRUD path does.
        state = client.app.state.eduport
        path = state.file_store.write("eth-K9p3", ent, "Body")
        upsert_entity(
            state.conn,
            "eth-K9p3", path, path.stat().st_mtime_ns,
            ent, "Body",
            schema=state.schema_store.current(),
        )

        # Delete the property from the schema → tier becomes orphaned.
        client.delete("/api/schema/types/university/properties/tier")
        # YAML still has it on disk:
        assert "tier:" in path.read_text(encoding="utf-8")

        # Purge:
        r = client.post(
            "/api/schema/types/university/properties/tier/purge_orphans"
        )
        assert r.status_code == 200
        body = r.json()
        assert body["rewritten"] == 1
        assert body["skipped"] == []
        # YAML no longer has the key:
        assert "tier:" not in path.read_text(encoding="utf-8")

    def test_skips_files_without_the_key(self, client: TestClient) -> None:
        from eduport.index.writer import upsert_entity
        from eduport.models import University

        # No tier property declared; entity has no tier — purge is a no-op.
        ent = University.model_validate({
            "tags": ["eduport-type/university"],
            "name": "ETH", "country": "Switzerland",
        })
        state = client.app.state.eduport
        path = state.file_store.write("eth-K9p3", ent, "Body")
        upsert_entity(
            state.conn, "eth-K9p3", path, path.stat().st_mtime_ns,
            ent, "Body", schema=state.schema_store.current(),
        )
        r = client.post(
            "/api/schema/types/university/properties/tier/purge_orphans"
        )
        assert r.status_code == 200
        assert r.json()["rewritten"] == 0
