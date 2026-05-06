from __future__ import annotations

import shutil
from pathlib import Path

from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel

from eduport.api.deps import AppState, get_state
from eduport.index.writer import upsert_entity
from eduport.parsers.entity import ParseError, parse_file

router = APIRouter(prefix="/trash", tags=["trash"])


def _safe_trash_path(state: AppState, name: str) -> Path:
    if "/" in name or "\\" in name or name in {"", ".", ".."}:
        raise HTTPException(status_code=400, detail="invalid trash item name")
    path = state.trash.trash_dir / name
    try:
        path.resolve().relative_to(state.trash.trash_dir.resolve())
    except ValueError:
        raise HTTPException(status_code=400, detail="invalid trash item name")
    if not path.exists() or not path.is_file() or path.name.endswith(".restore-from"):
        raise HTTPException(status_code=404, detail="trash item not found")
    return path


@router.get("")
def list_trash(state: AppState = Depends(get_state)) -> list[dict]:
    if not state.trash.trash_dir.exists():
        return []
    items = []
    for path in sorted(state.trash.trash_dir.iterdir(), key=lambda p: p.name.lower()):
        if not path.is_file() or path.name.endswith(".restore-from"):
            continue
        meta = path.with_suffix(path.suffix + ".restore-from")
        original = meta.read_text(encoding="utf-8") if meta.exists() else None
        stat = path.stat()
        items.append({
            "name": path.name,
            "path": str(path),
            "original_path": original,
            "size": stat.st_size,
            "modified": str(stat.st_mtime_ns),
        })
    return items


class TrashItemIn(BaseModel):
    name: str


@router.post("/restore")
def restore(payload: TrashItemIn, state: AppState = Depends(get_state)) -> dict:
    trashed = _safe_trash_path(state, payload.name)
    try:
        restored = state.trash.restore(trashed)
    except Exception as exc:
        raise HTTPException(status_code=400, detail=str(exc))

    result = parse_file(restored)
    if isinstance(result, ParseError):
        raise HTTPException(status_code=422, detail=result.message)

    upsert_entity(
        state.conn,
        file_id=restored.stem,
        path=restored,
        mtime_ns=restored.stat().st_mtime_ns,
        entity=result.entity,
        body=result.body,
    )
    return {"path": str(restored), "file_id": restored.stem}


@router.delete("/{name}", status_code=204)
def delete_trash_item(name: str, state: AppState = Depends(get_state)) -> None:
    path = _safe_trash_path(state, name)
    meta = path.with_suffix(path.suffix + ".restore-from")
    path.unlink()
    if meta.exists():
        meta.unlink()


@router.delete("", status_code=204)
def empty_trash(state: AppState = Depends(get_state)) -> None:
    if state.trash.trash_dir.exists():
        shutil.rmtree(state.trash.trash_dir)
