"""HTTP API for the user-managed property schema.

Endpoints:
    GET    /api/schema                                       — full schema
    GET    /api/schema/types/{type}                          — one type's schema
    POST   /api/schema/types/{type}/properties               — add property
    PATCH  /api/schema/types/{type}/properties/{key}         — edit allowed fields
    DELETE /api/schema/types/{type}/properties/{key}         — remove property
    POST   /api/schema/types/{type}/properties/{key}/purge_orphans — placeholder
    POST   /api/schema/templates/tier                        — convenience template

Schema-validation errors (collisions, immutability) return HTTP 409 with the
underlying message; payload-shape errors (bad property body) return 422 via
Pydantic's normal flow.
"""

from __future__ import annotations

from pathlib import Path as PathLib
from typing import Optional

from fastapi import APIRouter, Body, Depends, HTTPException, Path
from pydantic import BaseModel, ConfigDict, Field, ValidationError

from eduport.api.deps import AppState, get_state
from eduport.index.writer import reindex_all_properties, upsert_entity
from eduport.models import (
    EntitySchema,
    EntityType,
    Property,
    Schema,
    SelectOption,
)
from eduport.parsers.entity import _TYPE_TO_MODEL
from eduport.parsers.frontmatter import split
from eduport.store.schema_store import (
    PatchableFields,
    SchemaStoreError,
    builtin_keys,
)


def _reindex(state: AppState, schema: Schema) -> None:
    """Re-derive ``properties`` rows after a schema mutation."""
    reindex_all_properties(state.conn, schema)

router = APIRouter(prefix="/api/schema")


# ---- response shaping --------------------------------------------------------


def _entity_schema_payload(entity_type: EntityType, schema: Schema) -> dict:
    return {
        "entity_type": entity_type.value,
        "builtin_keys": sorted(builtin_keys(entity_type)),
        "properties": [p.model_dump(mode="json") for p in schema.for_type(entity_type).properties],
    }


def _full_schema_payload(schema: Schema) -> dict:
    return {
        "version": schema.version,
        "types": {
            t.value: _entity_schema_payload(t, schema) for t in EntityType
        },
    }


# ---- read --------------------------------------------------------------------


@router.get("")
def get_schema(state: AppState = Depends(get_state)) -> dict:
    return _full_schema_payload(state.schema_store.current())


@router.get("/types/{entity_type}")
def get_type_schema(
    entity_type: EntityType, state: AppState = Depends(get_state)
) -> dict:
    return _entity_schema_payload(entity_type, state.schema_store.current())


# ---- mutate ------------------------------------------------------------------


def _validate_property_body(body: dict) -> Property:
    """Round-trip a single property dict through EntitySchema's discriminated
    union, so all the per-type Pydantic validation runs the same way it
    does on file load.
    """
    try:
        wrapped = EntitySchema.model_validate({"properties": [body]})
    except ValidationError as exc:
        # include_context=False strips the raw Python exception from `ctx`
        # so the error list is JSON-serializable for FastAPI's response.
        raise HTTPException(
            status_code=422, detail=exc.errors(include_context=False, include_url=False)
        ) from exc
    return wrapped.properties[0]


@router.post("/types/{entity_type}/properties", status_code=201)
def add_property(
    entity_type: EntityType,
    body: dict = Body(...),
    state: AppState = Depends(get_state),
) -> dict:
    prop = _validate_property_body(body)
    try:
        new_schema = state.schema_store.add_property(entity_type, prop)
    except SchemaStoreError as exc:
        raise HTTPException(status_code=409, detail=str(exc)) from exc
    _reindex(state, new_schema)
    return _entity_schema_payload(entity_type, new_schema)


class PropertyPatchBody(BaseModel):
    model_config = ConfigDict(extra="forbid")

    name: Optional[str] = None
    description: Optional[str] = None
    required: Optional[bool] = None
    default: Optional[object] = None
    unit: Optional[str] = None
    options: Optional[list[SelectOption]] = None
    target_types: Optional[list[EntityType]] = Field(default=None, min_length=1)


@router.patch("/types/{entity_type}/properties/{key}")
def patch_property(
    entity_type: EntityType,
    key: str,
    body: PropertyPatchBody,
    state: AppState = Depends(get_state),
) -> dict:
    patch = PatchableFields(
        name=body.name,
        description=body.description,
        required=body.required,
        default=body.default,
        unit=body.unit,
        options=body.options,
        target_types=body.target_types,
    )
    try:
        new_schema = state.schema_store.patch_property(entity_type, key, patch)
    except SchemaStoreError as exc:
        # 404 when the property doesn't exist; 409 for conflicts.
        msg = str(exc)
        status = 404 if msg.startswith("no property") else 409
        raise HTTPException(status_code=status, detail=msg) from exc
    _reindex(state, new_schema)
    return _entity_schema_payload(entity_type, new_schema)


@router.delete("/types/{entity_type}/properties/{key}", status_code=200)
def delete_property(
    entity_type: EntityType,
    key: str,
    state: AppState = Depends(get_state),
) -> dict:
    try:
        new_schema = state.schema_store.delete_property(entity_type, key)
    except SchemaStoreError as exc:
        raise HTTPException(status_code=404, detail=str(exc)) from exc
    _reindex(state, new_schema)
    return _entity_schema_payload(entity_type, new_schema)


# ---- templates ---------------------------------------------------------------


class TierTemplateBody(BaseModel):
    model_config = ConfigDict(extra="forbid")
    types: list[EntityType] = Field(min_length=1)


_TIER_OPTIONS: list[dict] = [
    {"value": "reach", "label": "Reach", "color": "red"},
    {"value": "target", "label": "Target", "color": "yellow"},
    {"value": "safety", "label": "Safety", "color": "green"},
]


@router.post("/templates/tier", status_code=201)
def add_tier_template(
    body: TierTemplateBody = Body(...),
    state: AppState = Depends(get_state),
) -> dict:
    """Add a `tier` single-select to each requested entity type.

    Convenience: equivalent to N calls to POST /properties with the same body.
    Skips types that already have a `tier` property (idempotent for callers
    re-running the template).
    """
    results: dict[str, dict] = {}
    schema = state.schema_store.current()
    for entity_type in body.types:
        if schema.for_type(entity_type).property("tier") is not None:
            results[entity_type.value] = {"status": "exists"}
            continue
        prop_body = {
            "type": "single-select",
            "key": "tier",
            "name": "Tier",
            "description": "Reach / target / safety bucket",
            "options": _TIER_OPTIONS,
        }
        prop = _validate_property_body(prop_body)
        try:
            schema = state.schema_store.add_property(entity_type, prop)
        except SchemaStoreError as exc:
            raise HTTPException(status_code=409, detail=str(exc)) from exc
        results[entity_type.value] = {"status": "added"}
    _reindex(state, schema)
    return {"results": results, "schema": _full_schema_payload(schema)}


# ---- placeholder for purge_orphans (filled in when entity rewriting lands) ---


@router.post("/types/{entity_type}/properties/{key}/purge_orphans")
def purge_orphans(
    entity_type: EntityType,
    key: str = Path(...),
    state: AppState = Depends(get_state),
) -> dict:
    """Rewrite all entity files of this type that still carry an orphaned ``key``.

    Refuses if the key is currently declared in the schema — purge is for
    orphans only. Returns the count of files rewritten and any per-file
    skip reasons (missing file, parse error).
    """
    schema = state.schema_store.current()
    if schema.for_type(entity_type).property(key) is not None:
        raise HTTPException(
            status_code=409,
            detail=f"property {key!r} is currently declared on {entity_type.value}; "
            "delete it from the schema before purging orphans",
        )

    rewritten = 0
    skipped: list[dict] = []
    rows = state.conn.execute(
        "SELECT file_id, path FROM entities WHERE type = ?",
        (entity_type.value,),
    ).fetchall()

    for file_id, path_str in rows:
        path = PathLib(path_str)
        if not path.exists():
            skipped.append({"file_id": file_id, "reason": "missing"})
            continue
        try:
            raw = path.read_text(encoding="utf-8")
            fm, body = split(raw)
        except Exception as exc:
            skipped.append({"file_id": file_id, "reason": f"parse: {exc}"})
            continue
        if key not in fm:
            continue  # nothing to remove
        del fm[key]
        try:
            model_cls = _TYPE_TO_MODEL[entity_type]
            entity = model_cls.model_validate(fm)
        except Exception as exc:
            skipped.append({"file_id": file_id, "reason": f"validate: {exc}"})
            continue
        new_path = state.file_store.write(file_id, entity, body)
        upsert_entity(
            state.conn,
            file_id=file_id,
            path=new_path,
            mtime_ns=new_path.stat().st_mtime_ns,
            entity=entity,
            body=body,
            schema=schema,
        )
        rewritten += 1

    return {"rewritten": rewritten, "skipped": skipped}
