"""Tests for SchemaStore (load/seed/save/atomic + mutation API + validation)."""

from __future__ import annotations

from pathlib import Path

import pytest
import yaml

from eduport.models import (
    EntityType,
    NumberProperty,
    SelectOption,
    SingleSelectProperty,
    TextProperty,
    UrlProperty,
)
from eduport.models.schema import SCHEMA_VERSION
from eduport.store.schema_store import (
    PatchableFields,
    SchemaStore,
    SchemaStoreError,
    builtin_keys,
)


@pytest.fixture
def store(tmp_path: Path) -> SchemaStore:
    data = tmp_path / "data"
    data.mkdir()
    return SchemaStore(data)


class TestSeedAndLoad:
    def test_first_load_seeds_empty_schema(self, store: SchemaStore) -> None:
        assert not store.schema_path.exists()
        schema = store.load()
        assert store.schema_path.exists()
        assert schema.version == SCHEMA_VERSION
        for t in EntityType:
            assert schema.for_type(t).properties == []

    def test_load_caches(self, store: SchemaStore) -> None:
        first = store.load()
        # mutate file directly so we can detect cache vs reload
        store.schema_path.write_text("garbage", encoding="utf-8")
        cached = store.current()
        assert cached is first

    def test_reload_reads_disk(self, store: SchemaStore) -> None:
        store.load()
        # write a valid alternate schema and reload
        seed_payload = {
            "version": SCHEMA_VERSION,
            "types": {
                t.value: {"properties": []} for t in EntityType
            },
        }
        seed_payload["types"]["university"]["properties"].append(
            {
                "type": "text",
                "key": "ranking",
                "name": "Ranking",
            }
        )
        store.schema_path.write_text(yaml.safe_dump(seed_payload), encoding="utf-8")
        reloaded = store.reload()
        assert reloaded.for_type(EntityType.university).property("ranking") is not None


class TestAddProperty:
    def test_adds_to_empty(self, store: SchemaStore) -> None:
        prop = SingleSelectProperty(
            type="single-select",
            key="tier",
            name="Tier",
            options=[SelectOption(value="reach", label="Reach", color="red")],
        )
        store.load()
        schema = store.add_property(EntityType.university, prop)
        assert schema.for_type(EntityType.university).property("tier") is not None
        # persisted
        on_disk = yaml.safe_load(store.schema_path.read_text(encoding="utf-8"))
        uni = on_disk["types"]["university"]["properties"]
        assert uni[0]["key"] == "tier"

    def test_collision_with_builtin(self, store: SchemaStore) -> None:
        store.load()
        with pytest.raises(SchemaStoreError, match="built-in"):
            store.add_property(
                EntityType.university,
                TextProperty(type="text", key="country", name="Country"),
            )

    def test_collision_with_existing_custom(self, store: SchemaStore) -> None:
        store.load()
        store.add_property(
            EntityType.university,
            TextProperty(type="text", key="ranking", name="Ranking"),
        )
        with pytest.raises(SchemaStoreError, match="already exists"):
            store.add_property(
                EntityType.university,
                NumberProperty(type="number", key="ranking", name="Ranking 2"),
            )

    def test_same_key_on_different_types_is_fine(self, store: SchemaStore) -> None:
        store.load()
        store.add_property(
            EntityType.university,
            TextProperty(type="text", key="tier", name="Tier"),
        )
        store.add_property(
            EntityType.application,
            TextProperty(type="text", key="tier", name="Tier"),
        )
        schema = store.current()
        assert schema.for_type(EntityType.university).property("tier") is not None
        assert schema.for_type(EntityType.application).property("tier") is not None

    def test_builtin_keys_includes_inherited(self) -> None:
        # tags + name from BaseEntity should be reserved on every type
        for t in EntityType:
            assert "tags" in builtin_keys(t)
            assert "name" in builtin_keys(t)


class TestPatchProperty:
    def test_patch_name_and_description(self, store: SchemaStore) -> None:
        store.load()
        store.add_property(
            EntityType.university,
            TextProperty(type="text", key="notes", name="Notes"),
        )
        store.patch_property(
            EntityType.university,
            "notes",
            PatchableFields(name="Internal notes", description="private"),
        )
        prop = store.current().for_type(EntityType.university).property("notes")
        assert prop is not None
        assert prop.name == "Internal notes"
        assert prop.description == "private"

    def test_patch_unknown_field_rejected(self, store: SchemaStore) -> None:
        store.load()
        store.add_property(
            EntityType.university,
            TextProperty(type="text", key="notes", name="Notes"),
        )
        # unit is not patchable on text properties
        with pytest.raises(SchemaStoreError, match="not patchable"):
            store.patch_property(
                EntityType.university,
                "notes",
                PatchableFields(unit="kg"),
            )

    def test_patch_options_label_color(self, store: SchemaStore) -> None:
        store.load()
        store.add_property(
            EntityType.university,
            SingleSelectProperty(
                type="single-select",
                key="tier",
                name="Tier",
                options=[
                    SelectOption(value="reach", label="Reach", color="red"),
                    SelectOption(value="safety", label="Safety", color="green"),
                ],
            ),
        )
        store.patch_property(
            EntityType.university,
            "tier",
            PatchableFields(
                options=[
                    SelectOption(value="reach", label="Reaches", color="orange"),
                    SelectOption(value="safety", label="Safety!", color="green"),
                ],
            ),
        )
        prop = store.current().for_type(EntityType.university).property("tier")
        assert prop is not None
        assert prop.options[0].label == "Reaches"  # type: ignore[attr-defined]
        assert prop.options[0].color == "orange"  # type: ignore[attr-defined]

    def test_patch_options_add_new_option(self, store: SchemaStore) -> None:
        store.load()
        store.add_property(
            EntityType.university,
            SingleSelectProperty(
                type="single-select",
                key="tier",
                name="Tier",
                options=[SelectOption(value="reach", label="Reach")],
            ),
        )
        store.patch_property(
            EntityType.university,
            "tier",
            PatchableFields(
                options=[
                    SelectOption(value="reach", label="Reach"),
                    SelectOption(value="target", label="Target"),
                ],
            ),
        )
        prop = store.current().for_type(EntityType.university).property("tier")
        assert prop is not None
        values = [o.value for o in prop.options]  # type: ignore[attr-defined]
        assert values == ["reach", "target"]

    def test_patch_unknown_property_raises(self, store: SchemaStore) -> None:
        store.load()
        with pytest.raises(SchemaStoreError, match="no property"):
            store.patch_property(
                EntityType.university, "missing", PatchableFields(name="x")
            )


class TestDeleteProperty:
    def test_delete_existing(self, store: SchemaStore) -> None:
        store.load()
        store.add_property(
            EntityType.university,
            TextProperty(type="text", key="ranking", name="Ranking"),
        )
        store.delete_property(EntityType.university, "ranking")
        assert store.current().for_type(EntityType.university).property("ranking") is None

    def test_delete_unknown(self, store: SchemaStore) -> None:
        store.load()
        with pytest.raises(SchemaStoreError, match="no property"):
            store.delete_property(EntityType.university, "missing")


class TestAtomicWrite:
    def test_no_partial_file_on_failure(self, store: SchemaStore, monkeypatch) -> None:
        """If yaml dumping fails, the schema file must be left intact."""
        store.load()
        store.add_property(
            EntityType.university,
            TextProperty(type="text", key="ranking", name="Ranking"),
        )
        original_text = store.schema_path.read_text(encoding="utf-8")

        # Sabotage yaml.safe_dump to raise after the original file is on disk.
        import eduport.store.schema_store as ss

        def boom(*args, **kwargs):
            raise RuntimeError("boom")

        monkeypatch.setattr(ss.yaml, "safe_dump", boom)

        with pytest.raises(RuntimeError, match="boom"):
            store.add_property(
                EntityType.university,
                TextProperty(type="text", key="another", name="Another"),
            )
        assert store.schema_path.read_text(encoding="utf-8") == original_text

    def test_no_leftover_tempfiles_on_success(self, store: SchemaStore) -> None:
        store.load()
        store.add_property(
            EntityType.university,
            TextProperty(type="text", key="ranking", name="Ranking"),
        )
        leftovers = list(store.schema_dir.glob(".schema-*.yaml.tmp"))
        assert leftovers == []


class TestRoundTrip:
    def test_save_then_reload_preserves_property_types(self, store: SchemaStore) -> None:
        store.load()
        store.add_property(
            EntityType.university,
            SingleSelectProperty(
                type="single-select",
                key="tier",
                name="Tier",
                options=[
                    SelectOption(value="reach", label="Reach", color="red"),
                    SelectOption(value="safety", label="Safety", color="green"),
                ],
            ),
        )
        store.add_property(
            EntityType.university,
            UrlProperty(type="url", key="page", name="Page"),
        )
        # fresh store, same path
        fresh = SchemaStore(store.data_folder)
        loaded = fresh.load()
        uni = loaded.for_type(EntityType.university)
        assert isinstance(uni.property("tier"), SingleSelectProperty)
        assert isinstance(uni.property("page"), UrlProperty)

    def test_invalid_yaml_on_disk_raises(self, store: SchemaStore) -> None:
        store.load()
        store.schema_path.write_text("::: not yaml :::\n  - bad", encoding="utf-8")
        fresh = SchemaStore(store.data_folder)
        with pytest.raises(SchemaStoreError):
            fresh.load()

    def test_invalid_schema_on_disk_raises(self, store: SchemaStore) -> None:
        store.load()
        store.schema_path.write_text(
            yaml.safe_dump(
                {"version": 1, "types": {"university": {"properties": []}}}
            ),
            encoding="utf-8",
        )
        fresh = SchemaStore(store.data_folder)
        with pytest.raises(SchemaStoreError, match="failed validation"):
            fresh.load()
