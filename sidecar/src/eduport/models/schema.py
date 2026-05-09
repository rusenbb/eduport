"""Pydantic models for the user-managed property schema.

The schema file lives at ``<data folder>/.eduport/schema.yaml``. It is
*strict* (``extra="forbid"`` on every model) — this file is app-managed
through the schema editor; hand-editing remains supported but typos are
caught loudly. By contrast, entity files are *lenient* about custom keys
that match a declared property (see parsers/custom_fields.py).
"""

from __future__ import annotations

import re
from typing import Annotated, Literal, Optional, Union

from pydantic import (
    BaseModel,
    ConfigDict,
    Field,
    HttpUrl,
    field_validator,
    model_validator,
)

from eduport.models.base import EntityType, WikiLink

PropertyType = Literal[
    "text",
    "number",
    "date",
    "checkbox",
    "single-select",
    "multi-select",
    "url",
    "relation",
]

OptionColor = Literal[
    "gray",
    "red",
    "orange",
    "yellow",
    "green",
    "teal",
    "blue",
    "purple",
    "pink",
]

_KEY_RE = re.compile(r"^[a-z][a-z0-9_]{0,63}$")
_OPTION_VALUE_RE = re.compile(r"^[a-z0-9][a-z0-9_-]{0,63}$")


class SelectOption(BaseModel):
    model_config = ConfigDict(extra="forbid")

    value: str
    label: str = Field(min_length=1, max_length=120)
    color: OptionColor = "gray"

    @field_validator("value")
    @classmethod
    def _value_shape(cls, v: str) -> str:
        if not _OPTION_VALUE_RE.match(v):
            raise ValueError(
                "option value must match [a-z0-9][a-z0-9_-]{0,63} "
                f"(got {v!r})"
            )
        return v


class _PropertyBase(BaseModel):
    model_config = ConfigDict(extra="forbid")

    key: str
    name: str = Field(min_length=1, max_length=120)
    description: Optional[str] = Field(default=None, max_length=500)
    required: bool = False

    @field_validator("key")
    @classmethod
    def _key_shape(cls, v: str) -> str:
        if not _KEY_RE.match(v):
            raise ValueError(
                "property key must match [a-z][a-z0-9_]{0,63} "
                f"(got {v!r})"
            )
        return v


class TextProperty(_PropertyBase):
    type: Literal["text"]
    default: Optional[str] = None


class NumberProperty(_PropertyBase):
    type: Literal["number"]
    unit: Optional[str] = Field(default=None, max_length=32)
    default: Optional[float] = None


class DateProperty(_PropertyBase):
    type: Literal["date"]
    default: Optional[str] = None  # ISO date YYYY-MM-DD; checked at use sites

    @field_validator("default")
    @classmethod
    def _default_iso(cls, v: Optional[str]) -> Optional[str]:
        if v is None:
            return v
        from datetime import date

        try:
            date.fromisoformat(v)
        except ValueError as exc:
            raise ValueError(f"default must be an ISO date YYYY-MM-DD: {exc}") from exc
        return v


class CheckboxProperty(_PropertyBase):
    type: Literal["checkbox"]
    default: Optional[bool] = None


class SingleSelectProperty(_PropertyBase):
    type: Literal["single-select"]
    options: list[SelectOption] = Field(default_factory=list)
    default: Optional[str] = None

    @model_validator(mode="after")
    def _options_unique_and_default_valid(self):
        seen: set[str] = set()
        for opt in self.options:
            if opt.value in seen:
                raise ValueError(f"duplicate option value: {opt.value!r}")
            seen.add(opt.value)
        if self.default is not None and self.default not in seen:
            raise ValueError(
                f"default {self.default!r} is not among option values"
            )
        return self


class MultiSelectProperty(_PropertyBase):
    type: Literal["multi-select"]
    options: list[SelectOption] = Field(default_factory=list)
    default: Optional[list[str]] = None

    @model_validator(mode="after")
    def _options_unique_and_default_valid(self):
        seen: set[str] = set()
        for opt in self.options:
            if opt.value in seen:
                raise ValueError(f"duplicate option value: {opt.value!r}")
            seen.add(opt.value)
        if self.default is not None:
            for v in self.default:
                if v not in seen:
                    raise ValueError(
                        f"default {v!r} is not among option values"
                    )
        return self


class UrlProperty(_PropertyBase):
    type: Literal["url"]
    default: Optional[HttpUrl] = None


class RelationProperty(_PropertyBase):
    type: Literal["relation"]
    # Empty / None ⇒ any entity type permitted as the target.
    target_types: Optional[list[EntityType]] = None
    default: Optional[WikiLink] = None

    @field_validator("target_types")
    @classmethod
    def _no_empty_list(cls, v: Optional[list[EntityType]]) -> Optional[list[EntityType]]:
        if v is not None and len(v) == 0:
            raise ValueError(
                "target_types must be omitted entirely or contain at least one entity type"
            )
        return v


Property = Annotated[
    Union[
        TextProperty,
        NumberProperty,
        DateProperty,
        CheckboxProperty,
        SingleSelectProperty,
        MultiSelectProperty,
        UrlProperty,
        RelationProperty,
    ],
    Field(discriminator="type"),
]


class EntitySchema(BaseModel):
    model_config = ConfigDict(extra="forbid")

    properties: list[Property] = Field(default_factory=list)

    @model_validator(mode="after")
    def _keys_unique(self):
        seen: set[str] = set()
        for prop in self.properties:
            if prop.key in seen:
                raise ValueError(f"duplicate property key: {prop.key!r}")
            seen.add(prop.key)
        return self

    def property(self, key: str) -> Optional[Property]:
        return next((p for p in self.properties if p.key == key), None)


SCHEMA_VERSION = 1


class Schema(BaseModel):
    """The full user-managed schema. Every EntityType must have an entry."""

    model_config = ConfigDict(extra="forbid")

    version: int = SCHEMA_VERSION
    types: dict[EntityType, EntitySchema]

    @model_validator(mode="after")
    def _all_types_present(self):
        missing = set(EntityType) - set(self.types.keys())
        if missing:
            raise ValueError(
                "schema missing entries for entity types: "
                + ", ".join(sorted(t.value for t in missing))
            )
        return self

    @field_validator("version")
    @classmethod
    def _version_supported(cls, v: int) -> int:
        if v != SCHEMA_VERSION:
            raise ValueError(
                f"unsupported schema version {v}; this build expects {SCHEMA_VERSION}"
            )
        return v

    def for_type(self, entity_type: EntityType) -> EntitySchema:
        return self.types[entity_type]


def empty_schema() -> Schema:
    """Seed schema with all eight entity types and no properties."""
    return Schema(
        version=SCHEMA_VERSION,
        types={t: EntitySchema(properties=[]) for t in EntityType},
    )
