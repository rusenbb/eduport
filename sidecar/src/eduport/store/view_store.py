"""On-disk store for user-saved views.

Mirrors :class:`SchemaStore` exactly: load/seed/atomic save, mutation API,
in-memory cache, watcher integration. The file is a sibling of
``schema.yaml`` inside ``.eduport/``.
"""

from __future__ import annotations

import os
import tempfile
import threading
from pathlib import Path
from typing import Optional

import yaml
from pydantic import ValidationError

from eduport.models import (
    EntityType,
    TypeViews,
    View,
    ViewsFile,
    empty_views_file,
)
from eduport.slug import generate_slug
from eduport.store.schema_store import SCHEMA_DIR_NAME

VIEWS_FILENAME = "views.yaml"


class ViewStoreError(ValueError):
    """Raised on attempts to violate views-file invariants."""


class ViewStore:
    def __init__(self, data_folder: Path) -> None:
        self.data_folder = data_folder
        self._lock = threading.Lock()
        self._cached: Optional[ViewsFile] = None

    @property
    def views_dir(self) -> Path:
        return self.data_folder / SCHEMA_DIR_NAME

    @property
    def views_path(self) -> Path:
        return self.views_dir / VIEWS_FILENAME

    # ---- public API ------------------------------------------------------

    def load(self) -> ViewsFile:
        with self._lock:
            self._cached = self._load_locked()
            return self._cached

    def reload(self) -> ViewsFile:
        with self._lock:
            self._cached = None
            self._cached = self._load_locked()
            return self._cached

    def current(self) -> ViewsFile:
        with self._lock:
            if self._cached is None:
                self._cached = self._load_locked()
            return self._cached

    def add_view(self, entity_type: EntityType, view: View) -> ViewsFile:
        with self._lock:
            views_file = self._cached or self._load_locked()
            type_views = views_file.for_type(entity_type)
            if type_views.view(view.id) is not None:
                raise ViewStoreError(
                    f"view {view.id!r} already exists on {entity_type.value}"
                )
            new_views = list(type_views.views) + [view]
            new_file = self._with_views(views_file, entity_type, new_views)
            self._save_locked(new_file)
            self._cached = new_file
            return new_file

    def update_view(self, entity_type: EntityType, view_id: str, view: View) -> ViewsFile:
        if view.id != view_id:
            raise ViewStoreError("update payload id must match path id")
        with self._lock:
            views_file = self._cached or self._load_locked()
            type_views = views_file.for_type(entity_type)
            if type_views.view(view_id) is None:
                raise ViewStoreError(
                    f"no view {view_id!r} on {entity_type.value}"
                )
            new_views = [view if v.id == view_id else v for v in type_views.views]
            new_file = self._with_views(views_file, entity_type, new_views)
            self._save_locked(new_file)
            self._cached = new_file
            return new_file

    def delete_view(self, entity_type: EntityType, view_id: str) -> ViewsFile:
        with self._lock:
            views_file = self._cached or self._load_locked()
            type_views = views_file.for_type(entity_type)
            if type_views.view(view_id) is None:
                raise ViewStoreError(
                    f"no view {view_id!r} on {entity_type.value}"
                )
            new_views = [v for v in type_views.views if v.id != view_id]
            new_file = self._with_views(views_file, entity_type, new_views)
            self._save_locked(new_file)
            self._cached = new_file
            return new_file

    def reorder_views(
        self, entity_type: EntityType, ordered_ids: list[str]
    ) -> ViewsFile:
        with self._lock:
            views_file = self._cached or self._load_locked()
            type_views = views_file.for_type(entity_type)
            existing = {v.id: v for v in type_views.views}
            if set(ordered_ids) != set(existing.keys()):
                raise ViewStoreError(
                    "ordered_ids must contain exactly the existing view ids"
                )
            new_views = [existing[vid] for vid in ordered_ids]
            new_file = self._with_views(views_file, entity_type, new_views)
            self._save_locked(new_file)
            self._cached = new_file
            return new_file

    @staticmethod
    def generate_id(name: str, existing: set[str]) -> str:
        """Slug from name + 4-char alnum suffix (mirrors entity-file ids).

        Retries on collision; collision space is 62**4 ≈ 14.7M which is
        plenty for a per-entity-type view list (typically <50 views).
        """
        from eduport.ids import generate_id as gen_id

        slug = generate_slug(name) or "view"
        suffix = gen_id(lambda candidate: f"{slug}-{candidate}" in existing)
        return f"{slug}-{suffix}"

    # ---- internals -------------------------------------------------------

    def _load_locked(self) -> ViewsFile:
        path = self.views_path
        if not path.exists():
            seeded = empty_views_file()
            self._save_locked(seeded)
            return seeded
        text = path.read_text(encoding="utf-8")
        try:
            payload = yaml.safe_load(text) or {}
        except yaml.YAMLError as exc:
            raise ViewStoreError(f"views file is invalid YAML: {exc}") from exc
        try:
            return ViewsFile.model_validate(payload)
        except ValidationError as exc:
            raise ViewStoreError(f"views file failed validation: {exc}") from exc

    def _save_locked(self, views_file: ViewsFile) -> None:
        self.views_dir.mkdir(parents=True, exist_ok=True)
        payload = views_file.model_dump(mode="json", exclude_none=True)
        text = yaml.safe_dump(payload, sort_keys=False, allow_unicode=True)
        fd, tmp_path = tempfile.mkstemp(
            prefix=".views-", suffix=".yaml.tmp", dir=str(self.views_dir)
        )
        try:
            with os.fdopen(fd, "w", encoding="utf-8") as f:
                f.write(text)
            os.replace(tmp_path, self.views_path)
        except Exception:
            Path(tmp_path).unlink(missing_ok=True)
            raise

    @staticmethod
    def _with_views(
        views_file: ViewsFile, entity_type: EntityType, views: list[View]
    ) -> ViewsFile:
        new_types = dict(views_file.types)
        new_types[entity_type] = TypeViews(views=views)
        return ViewsFile(version=views_file.version, types=new_types)
