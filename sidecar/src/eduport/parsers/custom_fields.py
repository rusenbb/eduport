"""Lenient validator for user-defined custom properties on entities.

Built-in fields (Pydantic-typed) validate strictly during entity load.
This module covers the *custom* keys that arrive as `model_extra`: it
reads the schema, classifies each key/value, and produces a list of
:class:`ValueWarning`s describing problems. Wrong types and out-of-spec
selects produce warnings; nothing here ever raises.

The result is attached to the entity payload that the API returns, so
the frontend can render warning chips next to offending fields without
the file being rejected.
"""

from __future__ import annotations

from dataclasses import dataclass
from datetime import date
from enum import Enum
from typing import Any, Optional

from pydantic import HttpUrl, TypeAdapter, ValidationError

from eduport.models import (
    BaseEntity,
    EntitySchema,
    EntityType,
    Property,
    Schema,
    WikiLink,
)


class WarningKind(str, Enum):
    orphaned = "orphaned"  # key is not declared in schema
    type_mismatch = "type_mismatch"  # wrong type for declared property
    out_of_options = "out_of_options"  # select value not in options
    broken_link = "broken_link"  # relation target id not found in known set
    wrong_target_type = "wrong_target_type"  # relation target type not in target_types
    required_missing = "required_missing"  # required property has no value


@dataclass(frozen=True)
class ValueWarning:
    key: str
    kind: WarningKind
    message: str
    value: Any = None  # the offending value (for type_mismatch / out_of_options)


_HTTPURL = TypeAdapter(HttpUrl)
_WIKILINK = TypeAdapter(WikiLink)


def validate_custom_fields(
    entity: BaseEntity,
    schema: Schema,
    *,
    known_target_ids: Optional[dict[str, EntityType]] = None,
) -> list[ValueWarning]:
    """Validate custom keys on ``entity`` against ``schema``.

    Returns a list of warnings; never raises. ``known_target_ids`` is an
    optional id→entity-type map used to detect broken / wrong-type
    relations; if omitted, relation targets are not link-checked (the
    string shape is still validated).
    """
    type_schema = schema.for_type(entity.entity_type())
    declared = {p.key: p for p in type_schema.properties}
    extras: dict[str, Any] = dict(entity.model_extra or {})
    warnings: list[ValueWarning] = []

    # 1. Orphaned keys (present but undeclared).
    for key in extras:
        if key not in declared:
            warnings.append(
                ValueWarning(
                    key=key,
                    kind=WarningKind.orphaned,
                    message=f"{key!r} is not declared in the schema for {entity.entity_type().value}",
                )
            )

    # 2. Required-missing.
    for key, prop in declared.items():
        if prop.required and key not in extras:
            warnings.append(
                ValueWarning(
                    key=key,
                    kind=WarningKind.required_missing,
                    message=f"{key!r} is required but missing",
                )
            )

    # 3. Type / option / link checks for declared keys.
    for key, prop in declared.items():
        if key not in extras:
            continue
        value = extras[key]
        warnings.extend(_check_value(prop, value, known_target_ids))

    return warnings


def _check_value(
    prop: Property,
    value: Any,
    known_target_ids: Optional[dict[str, EntityType]],
) -> list[ValueWarning]:
    checker = _CHECKERS[prop.type]
    return checker(prop, value, known_target_ids)


def _check_text(prop: Property, value: Any, _: Any) -> list[ValueWarning]:
    if not isinstance(value, str):
        return [_type_mismatch(prop, value, "string")]
    return []


def _check_number(prop: Property, value: Any, _: Any) -> list[ValueWarning]:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        return [_type_mismatch(prop, value, "number")]
    return []


def _check_date(prop: Property, value: Any, _: Any) -> list[ValueWarning]:
    if isinstance(value, date):
        return []
    if isinstance(value, str):
        try:
            date.fromisoformat(value)
        except ValueError:
            return [_type_mismatch(prop, value, "ISO date YYYY-MM-DD")]
        return []
    return [_type_mismatch(prop, value, "ISO date YYYY-MM-DD")]


def _check_checkbox(prop: Property, value: Any, _: Any) -> list[ValueWarning]:
    if not isinstance(value, bool):
        return [_type_mismatch(prop, value, "boolean")]
    return []


def _check_single_select(prop: Property, value: Any, _: Any) -> list[ValueWarning]:
    if not isinstance(value, str):
        return [_type_mismatch(prop, value, "string (option value)")]
    valid = {o.value for o in prop.options}  # type: ignore[attr-defined]
    if value not in valid:
        return [
            ValueWarning(
                key=prop.key,
                kind=WarningKind.out_of_options,
                message=f"value {value!r} is not in the option list",
                value=value,
            )
        ]
    return []


def _check_multi_select(prop: Property, value: Any, _: Any) -> list[ValueWarning]:
    if not isinstance(value, list) or not all(isinstance(v, str) for v in value):
        return [_type_mismatch(prop, value, "list of strings (option values)")]
    valid = {o.value for o in prop.options}  # type: ignore[attr-defined]
    bad = [v for v in value if v not in valid]
    if bad:
        return [
            ValueWarning(
                key=prop.key,
                kind=WarningKind.out_of_options,
                message=f"values {bad!r} are not in the option list",
                value=bad,
            )
        ]
    return []


def _check_url(prop: Property, value: Any, _: Any) -> list[ValueWarning]:
    if not isinstance(value, str):
        return [_type_mismatch(prop, value, "URL string")]
    try:
        _HTTPURL.validate_python(value)
    except ValidationError:
        return [_type_mismatch(prop, value, "URL string")]
    return []


def _check_relation(
    prop: Property,
    value: Any,
    known_target_ids: Optional[dict[str, EntityType]],
) -> list[ValueWarning]:
    try:
        link = _WIKILINK.validate_python(value)
    except ValidationError:
        return [_type_mismatch(prop, value, "wikilink string [[target-id]]")]

    if known_target_ids is None:
        return []  # link-resolution not requested

    target_type = known_target_ids.get(link.target)
    if target_type is None:
        return [
            ValueWarning(
                key=prop.key,
                kind=WarningKind.broken_link,
                message=f"target {link.target!r} not found",
                value=value,
            )
        ]
    allowed = getattr(prop, "target_types", None)
    if allowed and target_type not in allowed:
        return [
            ValueWarning(
                key=prop.key,
                kind=WarningKind.wrong_target_type,
                message=(
                    f"target {link.target!r} is a {target_type.value}; "
                    f"property allows only {[t.value for t in allowed]}"
                ),
                value=value,
            )
        ]
    return []


def _type_mismatch(prop: Property, value: Any, expected: str) -> ValueWarning:
    return ValueWarning(
        key=prop.key,
        kind=WarningKind.type_mismatch,
        message=f"expected {expected}; got {type(value).__name__}",
        value=value,
    )


_CHECKERS = {
    "text": _check_text,
    "number": _check_number,
    "date": _check_date,
    "checkbox": _check_checkbox,
    "single-select": _check_single_select,
    "multi-select": _check_multi_select,
    "url": _check_url,
    "relation": _check_relation,
}


def warning_to_dict(w: ValueWarning) -> dict[str, Any]:
    """Serialize a ValueWarning for JSON API responses."""
    out: dict[str, Any] = {"key": w.key, "kind": w.kind.value, "message": w.message}
    if w.value is not None:
        out["value"] = w.value
    return out


__all__ = [
    "ValueWarning",
    "WarningKind",
    "validate_custom_fields",
    "warning_to_dict",
]


# silence unused import (used only for type narrowing in chip rendering)
_ = EntitySchema
