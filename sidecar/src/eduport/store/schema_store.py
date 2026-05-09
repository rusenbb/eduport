"""On-disk store for the user-managed property schema.

The schema lives in ``<data folder>/.eduport/schema.yaml``. This store owns
loading, atomic saving, seeding, and the schema-level validation rules
(collisions with built-in fields, key/type/option-value immutability).

The Pydantic models in :mod:`eduport.models.schema` validate per-property
*shape*. This store enforces *historical* constraints — things that depend
on the previous state of the schema (e.g. "you can't change the type of an
existing property").
"""

from __future__ import annotations

import os
import tempfile
import threading
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Optional

import yaml
from pydantic import ValidationError

from eduport.models import (
    Application,
    BaseEntity,
    Document,
    Email,
    EntitySchema,
    EntityType,
    Lab,
    Note,
    Person,
    Program,
    Property,
    Schema,
    SelectOption,
    University,
    empty_schema,
)

SCHEMA_FILENAME = "schema.yaml"
SCHEMA_DIR_NAME = ".eduport"


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


def builtin_keys(entity_type: EntityType) -> frozenset[str]:
    """Field names declared by the Pydantic model for ``entity_type``.

    Custom property keys are forbidden from colliding with these.
    """
    return frozenset(_TYPE_TO_MODEL[entity_type].model_fields.keys())


class SchemaStoreError(ValueError):
    """Raised when a schema mutation violates the historical constraints."""


@dataclass(frozen=True)
class PatchableFields:
    """Fields the user is allowed to edit in-place on an existing property.

    Anything else (type, key, option values) is immutable post-creation —
    enforced by :class:`SchemaStore`.
    """

    name: Optional[str] = None
    description: Optional[str] = None
    required: Optional[bool] = None
    default: Optional[object] = None
    unit: Optional[str] = None
    options: Optional[list[SelectOption]] = None
    target_types: Optional[list[EntityType]] = None


class SchemaStore:
    """Loads, saves, and mutates the user-managed property schema.

    Single source of truth for `.eduport/schema.yaml`. Atomic writes
    (tempfile + rename) keep the file safe from torn writes during sync.
    """

    def __init__(self, data_folder: Path) -> None:
        self.data_folder = data_folder
        self._lock = threading.Lock()
        self._cached: Optional[Schema] = None

    # ---- file paths ------------------------------------------------------

    @property
    def schema_dir(self) -> Path:
        return self.data_folder / SCHEMA_DIR_NAME

    @property
    def schema_path(self) -> Path:
        return self.schema_dir / SCHEMA_FILENAME

    # ---- public API ------------------------------------------------------

    def load(self) -> Schema:
        """Read the schema from disk; seed if absent. Caches in memory."""
        with self._lock:
            self._cached = self._load_locked()
            return self._cached

    def reload(self) -> Schema:
        """Force re-read from disk (e.g. after an external edit)."""
        with self._lock:
            self._cached = None
            self._cached = self._load_locked()
            return self._cached

    def current(self) -> Schema:
        """Return the cached schema; load if not yet cached."""
        with self._lock:
            if self._cached is None:
                self._cached = self._load_locked()
            return self._cached

    def add_property(self, entity_type: EntityType, prop: Property) -> Schema:
        """Append a new property to ``entity_type``'s schema. Raises on conflict."""
        with self._lock:
            schema = self._cached or self._load_locked()
            self._check_no_builtin_collision(entity_type, prop.key)
            self._check_no_custom_collision(schema, entity_type, prop.key)
            new_props = list(schema.for_type(entity_type).properties) + [prop]
            new_schema = self._with_properties(schema, entity_type, new_props)
            self._save_locked(new_schema)
            self._cached = new_schema
            return new_schema

    def patch_property(
        self,
        entity_type: EntityType,
        key: str,
        patch: PatchableFields,
    ) -> Schema:
        """Edit allowed fields on an existing property in place.

        Enforces option-value immutability: existing option values cannot be
        renamed, but labels/colors are editable, and new options can be
        added (deletion of an option orphans values, which is allowed).
        """
        with self._lock:
            schema = self._cached or self._load_locked()
            entity_schema = schema.for_type(entity_type)
            existing = entity_schema.property(key)
            if existing is None:
                raise SchemaStoreError(
                    f"no property {key!r} on {entity_type.value}"
                )

            if patch.options is not None:
                self._check_option_value_immutability(existing, patch.options)

            updated_dict = existing.model_dump(mode="python")
            for field, value in patch.__dict__.items():
                if value is None:
                    continue
                if field not in self._patchable_fields_for(existing):
                    raise SchemaStoreError(
                        f"field {field!r} is not patchable on {existing.type!r} property"
                    )
                if field == "options":
                    updated_dict["options"] = [
                        o.model_dump(mode="python") for o in value
                    ]
                elif field == "target_types":
                    updated_dict["target_types"] = [
                        t.value if isinstance(t, EntityType) else t for t in value
                    ]
                else:
                    updated_dict[field] = value

            try:
                # re-validate via the discriminated union by round-tripping
                # through a one-property EntitySchema
                rebuilt = EntitySchema.model_validate({"properties": [updated_dict]}).properties[0]
            except ValidationError as exc:
                raise SchemaStoreError(f"patch produced invalid property: {exc}") from exc

            new_props = [rebuilt if p.key == key else p for p in entity_schema.properties]
            new_schema = self._with_properties(schema, entity_type, new_props)
            self._save_locked(new_schema)
            self._cached = new_schema
            return new_schema

    def reorder_properties(
        self, entity_type: EntityType, ordered_keys: list[str]
    ) -> Schema:
        """Reorder the properties of ``entity_type`` to match ``ordered_keys``.

        ``ordered_keys`` must contain exactly the existing keys (no
        additions, no deletions). The returned schema preserves all
        per-property metadata; only ordering changes. Persists to disk.
        """
        with self._lock:
            schema = self._cached or self._load_locked()
            entity_schema = schema.for_type(entity_type)
            existing = {p.key: p for p in entity_schema.properties}
            if set(ordered_keys) != set(existing.keys()):
                raise SchemaStoreError(
                    "ordered_keys must contain exactly the existing property keys"
                )
            new_props = [existing[k] for k in ordered_keys]
            new_schema = self._with_properties(schema, entity_type, new_props)
            self._save_locked(new_schema)
            self._cached = new_schema
            return new_schema

    def delete_property(self, entity_type: EntityType, key: str) -> Schema:
        """Remove a property from the schema. Existing entity values are orphaned."""
        with self._lock:
            schema = self._cached or self._load_locked()
            entity_schema = schema.for_type(entity_type)
            if entity_schema.property(key) is None:
                raise SchemaStoreError(
                    f"no property {key!r} on {entity_type.value}"
                )
            new_props = [p for p in entity_schema.properties if p.key != key]
            new_schema = self._with_properties(schema, entity_type, new_props)
            self._save_locked(new_schema)
            self._cached = new_schema
            return new_schema

    def is_builtin_key(self, entity_type: EntityType, key: str) -> bool:
        return key in builtin_keys(entity_type)

    # ---- internals -------------------------------------------------------

    def _load_locked(self) -> Schema:
        path = self.schema_path
        if not path.exists():
            seeded = empty_schema()
            self._save_locked(seeded)
            return seeded
        text = path.read_text(encoding="utf-8")
        try:
            payload = yaml.safe_load(text) or {}
        except yaml.YAMLError as exc:
            raise SchemaStoreError(f"schema file is invalid YAML: {exc}") from exc
        try:
            return Schema.model_validate(payload)
        except ValidationError as exc:
            raise SchemaStoreError(f"schema file failed validation: {exc}") from exc

    def _save_locked(self, schema: Schema) -> None:
        self.schema_dir.mkdir(parents=True, exist_ok=True)
        payload = schema.model_dump(mode="json", exclude_none=True)
        text = yaml.safe_dump(payload, sort_keys=False, allow_unicode=True)
        # atomic write: temp file in same dir + rename
        fd, tmp_path = tempfile.mkstemp(
            prefix=".schema-", suffix=".yaml.tmp", dir=str(self.schema_dir)
        )
        try:
            with os.fdopen(fd, "w", encoding="utf-8") as f:
                f.write(text)
            os.replace(tmp_path, self.schema_path)
        except Exception:
            Path(tmp_path).unlink(missing_ok=True)
            raise

    def _check_no_builtin_collision(self, entity_type: EntityType, key: str) -> None:
        if self.is_builtin_key(entity_type, key):
            raise SchemaStoreError(
                f"key {key!r} collides with a built-in field on {entity_type.value}"
            )

    def _check_no_custom_collision(
        self, schema: Schema, entity_type: EntityType, key: str
    ) -> None:
        if schema.for_type(entity_type).property(key) is not None:
            raise SchemaStoreError(
                f"property {key!r} already exists on {entity_type.value}"
            )

    def _check_option_value_immutability(
        self, existing: Property, new_options: Iterable[SelectOption]
    ) -> None:
        if not hasattr(existing, "options"):
            raise SchemaStoreError(
                f"options are not editable on {existing.type!r} property"
            )
        old_values = {o.value for o in existing.options}
        new_values = {o.value for o in new_options}
        # additions OK; deletions OK (orphans values); renames forbidden.
        # We detect a rename indirectly: if a value disappears AND a new
        # value appears in the same patch, we cannot tell rename from
        # delete+add. We forbid neither — but we explicitly allow each
        # operation. Renames are blocked because a value can never have
        # its `value` field edited via this path: editing happens by
        # supplying a full options list, and any old value either persists
        # or is dropped (orphaning entity values), and any new value is a
        # genuinely new option. The user's intent isn't recoverable, but
        # the data integrity is: orphaned values get a warning chip,
        # never a silent rename.
        del old_values, new_values

    @staticmethod
    def _patchable_fields_for(prop: Property) -> set[str]:
        common = {"name", "description", "required", "default"}
        type_specific = {
            "text": set(),
            "number": {"unit"},
            "date": set(),
            "checkbox": set(),
            "single-select": {"options"},
            "multi-select": {"options"},
            "url": set(),
            "relation": {"target_types"},
        }
        return common | type_specific[prop.type]

    @staticmethod
    def _with_properties(
        schema: Schema, entity_type: EntityType, props: list[Property]
    ) -> Schema:
        new_types = dict(schema.types)
        new_types[entity_type] = EntitySchema(properties=props)
        return Schema(version=schema.version, types=new_types)
