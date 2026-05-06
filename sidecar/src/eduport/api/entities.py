from __future__ import annotations

import json
from pathlib import Path
from typing import Annotated, Optional

from fastapi import APIRouter, Depends, HTTPException, Query
from pydantic import BaseModel

from eduport.api.deps import AppState, get_state
from eduport.ids import generate_id
from eduport.index.reader import backlinks, list_entities
from eduport.index.writer import delete_entity, upsert_entity
from eduport.models import EntityType
from eduport.models.base import BaseEntity
from eduport.parsers.entity import _TYPE_TO_MODEL
from eduport.slug import generate_slug

router = APIRouter(prefix="/entities", tags=["entities"])


def _validate_type(type_: str) -> str:
    try:
        return EntityType(type_).value
    except ValueError:
        raise HTTPException(status_code=400, detail=f"unknown type: {type_!r}")


@router.get("/{type_}")
def list_(
    type_: str,
    tag: Annotated[Optional[list[str]], Query()] = None,
    state: AppState = Depends(get_state),
) -> list[dict]:
    type_ = _validate_type(type_)
    return list_entities(state.conn, type=type_, tags=tag or [])


@router.get("/{type_}/{file_id}")
def get_one(
    type_: str,
    file_id: str,
    state: AppState = Depends(get_state),
) -> dict:
    type_ = _validate_type(type_)
    row = state.conn.execute(
        "SELECT type, name, body, frontmatter FROM entities WHERE file_id = ? AND type = ?",
        (file_id, type_),
    ).fetchone()
    if row is None:
        raise HTTPException(status_code=404, detail="not found")
    return {
        "file_id": file_id,
        "type": row[0],
        "entity": json.loads(row[3]),
        "body": row[2],
        "backlinks": backlinks(state.conn, file_id),
    }


class EntityWriteIn(BaseModel):
    frontmatter: dict
    body: str = ""


@router.post("/{type_}", status_code=201)
def create(
    type_: str,
    payload: EntityWriteIn,
    state: AppState = Depends(get_state),
) -> dict:
    type_value = _validate_type(type_)
    model_cls = _TYPE_TO_MODEL[EntityType(type_value)]
    try:
        entity: BaseEntity = model_cls.model_validate(payload.frontmatter)
    except Exception as exc:
        raise HTTPException(status_code=422, detail=str(exc))

    slug = generate_slug(entity.name)
    existing_ids = {
        row[0] for row in state.conn.execute("SELECT file_id FROM entities")
    }
    new_id = generate_id(lambda candidate: f"{slug}-{candidate}" in existing_ids)
    file_id = f"{slug}-{new_id}"

    path = state.file_store.write(file_id, entity, payload.body)
    upsert_entity(
        state.conn,
        file_id=file_id,
        path=path,
        mtime_ns=path.stat().st_mtime_ns,
        entity=entity,
        body=payload.body,
    )
    return {"file_id": file_id}


@router.patch("/{type_}/{file_id}")
def update(
    type_: str,
    file_id: str,
    payload: EntityWriteIn,
    state: AppState = Depends(get_state),
) -> dict:
    type_value = _validate_type(type_)
    if not state.conn.execute(
        "SELECT 1 FROM entities WHERE file_id = ? AND type = ?",
        (file_id, type_value),
    ).fetchone():
        raise HTTPException(status_code=404, detail="not found")

    model_cls = _TYPE_TO_MODEL[EntityType(type_value)]
    try:
        entity: BaseEntity = model_cls.model_validate(payload.frontmatter)
    except Exception as exc:
        raise HTTPException(status_code=422, detail=str(exc))

    path = state.file_store.write(file_id, entity, payload.body)
    upsert_entity(
        state.conn,
        file_id=file_id,
        path=path,
        mtime_ns=path.stat().st_mtime_ns,
        entity=entity,
        body=payload.body,
    )
    return {"file_id": file_id}


@router.delete("/{type_}/{file_id}", status_code=204)
def delete(
    type_: str,
    file_id: str,
    state: AppState = Depends(get_state),
) -> None:
    type_value = _validate_type(type_)
    row = state.conn.execute(
        "SELECT path FROM entities WHERE file_id = ? AND type = ?",
        (file_id, type_value),
    ).fetchone()
    if row is None:
        raise HTTPException(status_code=404, detail="not found")
    path = Path(row[0])
    if path.exists():
        state.trash.trash(path)
        state.file_store.delete_marker(path)
    delete_entity(state.conn, file_id)
