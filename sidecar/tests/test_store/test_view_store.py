"""Tests for ViewStore."""

from __future__ import annotations

from pathlib import Path

import pytest
import yaml

from eduport.models import EntityType, TypeViews, View, ViewFilter, empty_views_file
from eduport.models.view import VIEWS_VERSION
from eduport.store.view_store import ViewStore, ViewStoreError


@pytest.fixture
def store(tmp_path: Path) -> ViewStore:
    data = tmp_path / "data"
    data.mkdir()
    return ViewStore(data)


class TestSeedAndLoad:
    def test_first_load_seeds(self, store: ViewStore) -> None:
        assert not store.views_path.exists()
        f = store.load()
        assert store.views_path.exists()
        assert f.version == VIEWS_VERSION
        for t in EntityType:
            assert f.for_type(t).views == []

    def test_caches(self, store: ViewStore) -> None:
        first = store.load()
        store.views_path.write_text("garbage", encoding="utf-8")
        cached = store.current()
        assert cached is first

    def test_reload_reads_disk(self, store: ViewStore) -> None:
        store.load()
        seed = empty_views_file()
        seed.types[EntityType.university] = TypeViews(
            views=[View(id="a-1111", name="A")]
        )
        store.views_path.write_text(yaml.safe_dump(seed.model_dump(mode="json", exclude_none=True)))
        reloaded = store.reload()
        assert reloaded.for_type(EntityType.university).view("a-1111") is not None


class TestAddView:
    def test_add_persists(self, store: ViewStore) -> None:
        store.load()
        v = View(id="reach-A1B2", name="Reach schools", kind="board")
        store.add_view(EntityType.university, v)
        on_disk = yaml.safe_load(store.views_path.read_text())
        assert on_disk["types"]["university"]["views"][0]["id"] == "reach-A1B2"

    def test_duplicate_id_rejected(self, store: ViewStore) -> None:
        store.load()
        store.add_view(EntityType.university, View(id="x-AAAA", name="X"))
        with pytest.raises(ViewStoreError, match="already exists"):
            store.add_view(EntityType.university, View(id="x-AAAA", name="X"))

    def test_same_id_on_different_types_is_fine(self, store: ViewStore) -> None:
        store.load()
        store.add_view(EntityType.university, View(id="x-AAAA", name="X"))
        store.add_view(EntityType.application, View(id="x-AAAA", name="X"))


class TestUpdateView:
    def test_replaces_in_place(self, store: ViewStore) -> None:
        store.load()
        store.add_view(EntityType.university, View(id="x-AAAA", name="X"))
        new = View(
            id="x-AAAA",
            name="X renamed",
            kind="table",
            filter=ViewFilter(text={"tier": "reach"}),
        )
        store.update_view(EntityType.university, "x-AAAA", new)
        v = store.current().for_type(EntityType.university).view("x-AAAA")
        assert v is not None
        assert v.name == "X renamed"
        assert v.kind == "table"
        assert v.filter.text == {"tier": "reach"}

    def test_id_must_match_path(self, store: ViewStore) -> None:
        store.load()
        store.add_view(EntityType.university, View(id="x-AAAA", name="X"))
        with pytest.raises(ViewStoreError):
            store.update_view(
                EntityType.university,
                "x-AAAA",
                View(id="other-BBBB", name="Y"),
            )

    def test_unknown_view_raises(self, store: ViewStore) -> None:
        store.load()
        with pytest.raises(ViewStoreError, match="no view"):
            store.update_view(
                EntityType.university,
                "missing-AAAA",
                View(id="missing-AAAA", name="X"),
            )


class TestDeleteAndReorder:
    def test_delete(self, store: ViewStore) -> None:
        store.load()
        store.add_view(EntityType.university, View(id="x-AAAA", name="X"))
        store.delete_view(EntityType.university, "x-AAAA")
        assert store.current().for_type(EntityType.university).view("x-AAAA") is None

    def test_delete_unknown(self, store: ViewStore) -> None:
        store.load()
        with pytest.raises(ViewStoreError, match="no view"):
            store.delete_view(EntityType.university, "missing-AAAA")

    def test_reorder(self, store: ViewStore) -> None:
        store.load()
        store.add_view(EntityType.university, View(id="a-AAAA", name="A"))
        store.add_view(EntityType.university, View(id="b-BBBB", name="B"))
        store.add_view(EntityType.university, View(id="c-CCCC", name="C"))
        store.reorder_views(EntityType.university, ["c-CCCC", "a-AAAA", "b-BBBB"])
        ids = [v.id for v in store.current().for_type(EntityType.university).views]
        assert ids == ["c-CCCC", "a-AAAA", "b-BBBB"]

    def test_reorder_mismatch_rejected(self, store: ViewStore) -> None:
        store.load()
        store.add_view(EntityType.university, View(id="a-AAAA", name="A"))
        with pytest.raises(ViewStoreError, match="must contain"):
            store.reorder_views(EntityType.university, ["a-AAAA", "ghost-XXXX"])


class TestIdGeneration:
    def test_generate_id_unique(self, store: ViewStore) -> None:
        existing: set[str] = set()
        for _ in range(20):
            new = store.generate_id("My View Name", existing)
            assert new not in existing
            assert new.startswith("my-view-name-")
            existing.add(new)


class TestAtomicWrite:
    def test_no_partial_file_on_failure(self, store: ViewStore, monkeypatch) -> None:
        store.load()
        store.add_view(EntityType.university, View(id="a-AAAA", name="A"))
        original = store.views_path.read_text()

        import eduport.store.view_store as vs

        def boom(*args, **kwargs):
            raise RuntimeError("boom")

        monkeypatch.setattr(vs.yaml, "safe_dump", boom)
        with pytest.raises(RuntimeError, match="boom"):
            store.add_view(EntityType.university, View(id="b-BBBB", name="B"))
        assert store.views_path.read_text() == original
