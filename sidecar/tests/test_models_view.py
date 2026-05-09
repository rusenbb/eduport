"""Tests for the saved-views Pydantic models (models/view.py)."""

from __future__ import annotations

import pytest
from pydantic import ValidationError

from eduport.models import EntityType, TypeViews, View, ViewFilter, ViewsFile, empty_views_file
from eduport.models.view import VIEWS_VERSION


class TestView:
    def test_minimum_view(self) -> None:
        v = View(id="my-view-A1B2", name="My view")
        assert v.kind == "list"
        assert v.sort_dir == "asc"
        assert v.filter.text == {}

    @pytest.mark.parametrize(
        "vid",
        ["", "x" * 200, "_starts-with-underscore", "-leading-dash"],
    )
    def test_invalid_id_rejected(self, vid: str) -> None:
        with pytest.raises(ValidationError):
            View(id=vid, name="x")

    def test_filter_carries_through(self) -> None:
        v = View(
            id="x-A1B2",
            name="X",
            filter=ViewFilter(
                text={"tier": "reach"},
                num={"rank": (1.0, 5.0)},
                date={"deadline": ("2026-01-01", "2026-12-31")},
            ),
        )
        assert v.filter.text == {"tier": "reach"}


class TestTypeViews:
    def test_unique_ids(self) -> None:
        with pytest.raises(ValidationError, match="duplicate view id"):
            TypeViews(
                views=[
                    View(id="a-1111", name="A"),
                    View(id="a-1111", name="A2"),
                ]
            )

    def test_view_lookup(self) -> None:
        tv = TypeViews(views=[View(id="a-1111", name="A")])
        assert tv.view("a-1111") is not None
        assert tv.view("missing") is None


class TestViewsFile:
    def test_empty_has_all_types(self) -> None:
        f = empty_views_file()
        assert set(f.types.keys()) == set(EntityType)
        for tv in f.types.values():
            assert tv.views == []

    def test_missing_type_rejected(self) -> None:
        with pytest.raises(ValidationError, match="missing entries"):
            ViewsFile(
                version=VIEWS_VERSION,
                types={EntityType.university: TypeViews(views=[])},
            )

    def test_unsupported_version_rejected(self) -> None:
        with pytest.raises(ValidationError, match="unsupported views version"):
            ViewsFile.model_validate(
                {"version": 999, "types": {t.value: {"views": []} for t in EntityType}}
            )

    def test_extra_field_rejected(self) -> None:
        with pytest.raises(ValidationError):
            ViewsFile.model_validate(
                {
                    "version": VIEWS_VERSION,
                    "types": {t.value: {"views": []} for t in EntityType},
                    "rogue": "no",
                }
            )

    def test_round_trip(self) -> None:
        original = empty_views_file()
        original.types[EntityType.university] = TypeViews(
            views=[
                View(
                    id="reaches-A1B2",
                    name="Reach schools",
                    kind="board",
                    filter=ViewFilter(text={"tier": "reach"}),
                    group_by_key="tier",
                )
            ]
        )
        # Re-validate via dict round-trip
        restored = ViewsFile.model_validate(original.model_dump(mode="json", exclude_none=True))
        assert restored == original

    def test_for_type(self) -> None:
        f = empty_views_file()
        f.types[EntityType.application] = TypeViews(
            views=[View(id="x-AAAA", name="X")]
        )
        assert f.for_type(EntityType.application).views[0].id == "x-AAAA"
