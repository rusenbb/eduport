"""HTTP API for user-saved views.

Endpoints:
    GET    /api/views                                — full views file
    GET    /api/views/types/{type}                   — one type's views
    POST   /api/views/types/{type}                   — create (id auto-generated)
    PUT    /api/views/types/{type}/{id}              — full update (replace)
    DELETE /api/views/types/{type}/{id}              — remove
    POST   /api/views/types/{type}/reorder           — drag-reorder

Validation errors map to 422 (Pydantic body) or 409 (store invariants).
"""

from __future__ import annotations

from fastapi import APIRouter, Body, Depends, HTTPException
from pydantic import BaseModel, ConfigDict, Field, ValidationError

from eduport.api.deps import AppState, get_state
from eduport.models import (
    EntityType,
    SortDir,
    View,
    ViewFilter,
    ViewKind,
    ViewsFile,
)
from eduport.store.view_store import ViewStoreError

router = APIRouter(prefix="/api/views")


def _type_payload(entity_type: EntityType, views_file: ViewsFile) -> dict:
    return {
        "entity_type": entity_type.value,
        "views": [v.model_dump(mode="json", exclude_none=True) for v in views_file.for_type(entity_type).views],
    }


def _full_payload(views_file: ViewsFile) -> dict:
    return {
        "version": views_file.version,
        "types": {
            t.value: _type_payload(t, views_file) for t in EntityType
        },
    }


class CreateViewBody(BaseModel):
    """Body for POST — id is generated server-side."""

    model_config = ConfigDict(extra="forbid")

    name: str = Field(min_length=1, max_length=120)
    kind: ViewKind = "list"
    filter: ViewFilter = Field(default_factory=ViewFilter)
    sort_key: str | None = None
    sort_dir: SortDir = "asc"
    group_by_key: str | None = None
    columns: list[str] | None = None
    card_properties: list[str] | None = None


class UpdateViewBody(BaseModel):
    """Body for PUT — full replacement of all editable fields. id stays fixed."""

    model_config = ConfigDict(extra="forbid")

    name: str = Field(min_length=1, max_length=120)
    kind: ViewKind
    filter: ViewFilter = Field(default_factory=ViewFilter)
    sort_key: str | None = None
    sort_dir: SortDir = "asc"
    group_by_key: str | None = None
    columns: list[str] | None = None
    card_properties: list[str] | None = None


class ReorderBody(BaseModel):
    model_config = ConfigDict(extra="forbid")
    ordered_ids: list[str]


# ---- routes ----------------------------------------------------------------


@router.get("")
def get_all(state: AppState = Depends(get_state)) -> dict:
    return _full_payload(state.view_store.current())


@router.get("/types/{entity_type}")
def get_for_type(
    entity_type: EntityType, state: AppState = Depends(get_state)
) -> dict:
    return _type_payload(entity_type, state.view_store.current())


@router.post("/types/{entity_type}", status_code=201)
def create_view(
    entity_type: EntityType,
    body: CreateViewBody = Body(...),
    state: AppState = Depends(get_state),
) -> dict:
    existing_ids = {v.id for v in state.view_store.current().for_type(entity_type).views}
    new_id = state.view_store.generate_id(body.name, existing_ids)
    try:
        view = View(
            id=new_id,
            name=body.name,
            kind=body.kind,
            filter=body.filter,
            sort_key=body.sort_key,
            sort_dir=body.sort_dir,
            group_by_key=body.group_by_key,
            columns=body.columns,
            card_properties=body.card_properties,
        )
    except ValidationError as exc:
        raise HTTPException(
            status_code=422,
            detail=exc.errors(include_context=False, include_url=False),
        ) from exc
    try:
        new_file = state.view_store.add_view(entity_type, view)
    except ViewStoreError as exc:
        raise HTTPException(status_code=409, detail=str(exc)) from exc
    return {"view": view.model_dump(mode="json", exclude_none=True), "type_views": _type_payload(entity_type, new_file)}


@router.put("/types/{entity_type}/{view_id}")
def update_view(
    entity_type: EntityType,
    view_id: str,
    body: UpdateViewBody = Body(...),
    state: AppState = Depends(get_state),
) -> dict:
    try:
        view = View(
            id=view_id,
            name=body.name,
            kind=body.kind,
            filter=body.filter,
            sort_key=body.sort_key,
            sort_dir=body.sort_dir,
            group_by_key=body.group_by_key,
            columns=body.columns,
            card_properties=body.card_properties,
        )
    except ValidationError as exc:
        raise HTTPException(
            status_code=422,
            detail=exc.errors(include_context=False, include_url=False),
        ) from exc
    try:
        new_file = state.view_store.update_view(entity_type, view_id, view)
    except ViewStoreError as exc:
        msg = str(exc)
        status = 404 if msg.startswith("no view") else 409
        raise HTTPException(status_code=status, detail=msg) from exc
    return {"view": view.model_dump(mode="json", exclude_none=True), "type_views": _type_payload(entity_type, new_file)}


@router.delete("/types/{entity_type}/{view_id}")
def delete_view(
    entity_type: EntityType,
    view_id: str,
    state: AppState = Depends(get_state),
) -> dict:
    try:
        new_file = state.view_store.delete_view(entity_type, view_id)
    except ViewStoreError as exc:
        raise HTTPException(status_code=404, detail=str(exc)) from exc
    return _type_payload(entity_type, new_file)


@router.post("/types/{entity_type}/reorder")
def reorder_views(
    entity_type: EntityType,
    body: ReorderBody = Body(...),
    state: AppState = Depends(get_state),
) -> dict:
    try:
        new_file = state.view_store.reorder_views(entity_type, body.ordered_ids)
    except ViewStoreError as exc:
        raise HTTPException(status_code=409, detail=str(exc)) from exc
    return _type_payload(entity_type, new_file)
