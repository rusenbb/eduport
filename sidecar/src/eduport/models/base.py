from __future__ import annotations

import re
from enum import Enum
from typing import Optional

from pydantic import (
    BaseModel,
    ConfigDict,
    Field,
    HttpUrl,
    field_validator,
    model_serializer,
    model_validator,
)


class EntityType(str, Enum):
    university = "university"
    lab = "lab"
    person = "person"
    program = "program"
    application = "application"
    document = "document"
    email = "email"
    note = "note"


_WIKILINK_RE = re.compile(r"^\[\[([^\]\[]+)\]\]$")


class WikiLink(BaseModel):
    """A `[[target]]` reference. `target` is the filename stem (no .md, no brackets).

    Round-trips through YAML/JSON as the bracketed string form so that the
    surrounding entity's serialized frontmatter matches the on-disk text.
    """

    model_config = ConfigDict(frozen=True)
    target: str

    @model_validator(mode="before")
    @classmethod
    def _accept_string(cls, value):
        if isinstance(value, str):
            m = _WIKILINK_RE.match(value)
            if not m:
                raise ValueError(f"Not a wikilink: {value!r}")
            return {"target": m.group(1).strip()}
        return value

    @model_serializer
    def _serialize(self) -> str:
        return f"[[{self.target}]]"

    def __str__(self) -> str:
        return f"[[{self.target}]]"


class LinkResource(BaseModel):
    model_config = ConfigDict(extra="forbid")
    label: str
    url: HttpUrl


class EmailResource(BaseModel):
    model_config = ConfigDict(extra="forbid")
    label: str
    email: str
    person: Optional[WikiLink] = None


_TYPE_PREFIX = "eduport-type/"
_DOCTYPE_PREFIX = "eduport-doctype/"


class BaseEntity(BaseModel):
    # Custom user-defined properties (see models/schema.py and parsers/custom_fields.py)
    # arrive as extra YAML keys and are kept on the model via Pydantic's `model_extra`.
    # The schema layer validates them; built-in fields below still validate strictly.
    model_config = ConfigDict(extra="allow")

    tags: list[str] = Field(default_factory=list)
    name: str

    def entity_type(self) -> EntityType:
        for tag in self.tags:
            if tag.startswith(_TYPE_PREFIX):
                return EntityType(tag[len(_TYPE_PREFIX):])
        raise ValueError("missing eduport-type/* tag")

    def user_tags(self) -> list[str]:
        """Return tags excluding app-managed reserved prefixes (`eduport-type/*`, `eduport-doctype/*`)."""
        return [
            t for t in self.tags
            if not t.startswith(_TYPE_PREFIX) and not t.startswith(_DOCTYPE_PREFIX)
        ]

    @field_validator("tags")
    @classmethod
    def _has_type_tag(cls, tags: list[str]) -> list[str]:
        if not any(t.startswith(_TYPE_PREFIX) for t in tags):
            raise ValueError("entity must have an eduport-type/* tag")
        return tags
