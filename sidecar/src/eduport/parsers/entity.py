from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Union

from pydantic import ValidationError

from eduport.models import (
    Application,
    BaseEntity,
    Document,
    Email,
    EntityType,
    Lab,
    Note,
    Person,
    Program,
    University,
)
from eduport.parsers.frontmatter import FrontmatterError, split

_TYPE_TO_MODEL: dict[EntityType, type[BaseEntity]] = {
    EntityType.university: University,
    EntityType.lab: Lab,
    EntityType.person: Person,
    EntityType.program: Program,
    EntityType.application: Application,
    EntityType.document: Document,
    EntityType.email: Email,
    EntityType.note: Note,
}


@dataclass(frozen=True)
class ParseError:
    path: Path
    message: str


@dataclass(frozen=True)
class ParsedEntity:
    """Wraps a typed model + raw body + path."""
    entity: BaseEntity
    body: str
    path: Path


ParseResult = Union[ParsedEntity, ParseError]


def parse_file(path: Path) -> ParseResult:
    try:
        raw = path.read_text(encoding="utf-8")
    except OSError as exc:
        return ParseError(path=path, message=f"read failed: {exc}")

    try:
        fm, body = split(raw)
    except FrontmatterError as exc:
        return ParseError(path=path, message=f"frontmatter error: {exc}")

    if not fm:
        return ParseError(path=path, message="missing frontmatter")

    fm.setdefault("name", path.stem)

    type_tag = next(
        (t for t in fm.get("tags", []) if t.startswith("eduport-type/")),
        None,
    )
    if type_tag is None:
        return ParseError(path=path, message="missing eduport-type/* tag")

    type_value = type_tag.removeprefix("eduport-type/")
    try:
        entity_type = EntityType(type_value)
    except ValueError:
        return ParseError(path=path, message=f"unknown entity type: {type_value!r}")

    model_cls = _TYPE_TO_MODEL[entity_type]
    try:
        entity = model_cls.model_validate(fm)
    except ValidationError as exc:
        return ParseError(path=path, message=f"validation error: {exc}")

    return ParsedEntity(entity=entity, body=body, path=path)
