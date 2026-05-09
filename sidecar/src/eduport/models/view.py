"""Pydantic models for user-saved list/table/board views.

A view captures a configuration of a list-style surface — entity type,
filter, sort, group-by, view kind, and view-kind-specific settings (column
selection for tables, card-property selection for boards).

Views are stored in ``<data folder>/.eduport/views.yaml`` and treated as
strict (``extra="forbid"``) like the schema file. Hand-editing remains
supported but typos are caught loudly.
"""

from __future__ import annotations

import re
from typing import Literal, Optional

from pydantic import BaseModel, ConfigDict, Field, field_validator, model_validator

from eduport.models.base import EntityType

ViewKind = Literal["list", "table", "board"]
SortDir = Literal["asc", "desc"]

VIEWS_VERSION = 1

# Lowercase slug + dash + 4-char alphanumeric suffix (mirrors entity-file ids).
# Permissive: any mix of lowercase letters, digits, dashes, underscores, plus
# uppercase letters in the id-suffix tail.
_ID_RE = re.compile(r"^[a-z0-9][a-zA-Z0-9_-]{0,127}$")


class ViewFilter(BaseModel):
    """Mirrors the frontend PropertyFilters shape; what /api/properties/filter
    accepts when assembled into a query string.
    """

    model_config = ConfigDict(extra="forbid")

    text: dict[str, str] = Field(default_factory=dict)
    num: dict[str, tuple[Optional[float], Optional[float]]] = Field(default_factory=dict)
    date: dict[str, tuple[Optional[str], Optional[str]]] = Field(default_factory=dict)


class View(BaseModel):
    model_config = ConfigDict(extra="forbid")

    id: str
    name: str = Field(min_length=1, max_length=120)
    kind: ViewKind = "list"
    filter: ViewFilter = Field(default_factory=ViewFilter)
    sort_key: Optional[str] = None
    sort_dir: SortDir = "asc"
    group_by_key: Optional[str] = None

    # Table view: which property keys to render as columns. Order matters.
    # `None` = use a sensible default (all single-line types up to a cap).
    columns: Optional[list[str]] = None

    # Board view: which property keys to render on each card (below name).
    card_properties: Optional[list[str]] = None

    @field_validator("id")
    @classmethod
    def _id_shape(cls, v: str) -> str:
        if not _ID_RE.match(v):
            raise ValueError(
                "view id must match [a-z0-9][a-z0-9_-]{0,127} "
                f"(got {v!r})"
            )
        return v


class TypeViews(BaseModel):
    model_config = ConfigDict(extra="forbid")

    views: list[View] = Field(default_factory=list)

    @model_validator(mode="after")
    def _ids_unique(self):
        seen: set[str] = set()
        for v in self.views:
            if v.id in seen:
                raise ValueError(f"duplicate view id: {v.id!r}")
            seen.add(v.id)
        return self

    def view(self, view_id: str) -> Optional[View]:
        return next((v for v in self.views if v.id == view_id), None)


class ViewsFile(BaseModel):
    """Top-level shape of `.eduport/views.yaml`. Every entity type must
    have an entry — same convention as the schema file.
    """

    model_config = ConfigDict(extra="forbid")

    version: int = VIEWS_VERSION
    types: dict[EntityType, TypeViews]

    @field_validator("version")
    @classmethod
    def _version_supported(cls, v: int) -> int:
        if v != VIEWS_VERSION:
            raise ValueError(
                f"unsupported views version {v}; this build expects {VIEWS_VERSION}"
            )
        return v

    @model_validator(mode="after")
    def _all_types_present(self):
        missing = set(EntityType) - set(self.types.keys())
        if missing:
            raise ValueError(
                "views file missing entries for entity types: "
                + ", ".join(sorted(t.value for t in missing))
            )
        return self

    def for_type(self, entity_type: EntityType) -> TypeViews:
        return self.types[entity_type]


def empty_views_file() -> ViewsFile:
    """Seed views file with all eight entity types and no views."""
    return ViewsFile(
        version=VIEWS_VERSION,
        types={t: TypeViews(views=[]) for t in EntityType},
    )
