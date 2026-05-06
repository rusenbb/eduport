from pathlib import Path

from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel

from eduport.api.deps import AppState, get_state
from eduport.index.writer import upsert_entity
from eduport.parsers.entity import ParseError, parse_file
from eduport.parsers.frontmatter import split as split_frontmatter

router = APIRouter()


class ToggleIn(BaseModel):
    file_id: str
    line: int
    checked: bool


@router.post("/checkbox/toggle")
def toggle(payload: ToggleIn, state: AppState = Depends(get_state)) -> dict:
    row = state.conn.execute(
        "SELECT path FROM entities WHERE file_id = ?", (payload.file_id,)
    ).fetchone()
    if row is None:
        raise HTTPException(status_code=404, detail="entity not found")
    path = Path(row[0])
    text = path.read_text(encoding="utf-8")
    # Use the same body extraction as the reader serves the frontend, so the
    # caller's body-relative line index lines up with what we patch on disk.
    _, body = split_frontmatter(text)
    body_offset = len(text) - len(body)
    body_lines = body.split("\n")
    if payload.line < 0 or payload.line >= len(body_lines):
        raise HTTPException(status_code=400, detail="line out of range")
    line = body_lines[payload.line]
    new_marker = "[x]" if payload.checked else "[ ]"
    if line.startswith("- [ ]"):
        body_lines[payload.line] = "- " + new_marker + line[len("- [ ]"):]
    elif line.startswith("- [x]") or line.startswith("- [X]"):
        body_lines[payload.line] = "- " + new_marker + line[len("- [x]"):]
    else:
        raise HTTPException(status_code=400, detail="line is not a checkbox")
    new_body = "\n".join(body_lines)
    new_text = text[:body_offset] + new_body
    path.write_text(new_text, encoding="utf-8")
    state.file_store.delete_marker(path)
    result = parse_file(path)
    if not isinstance(result, ParseError):
        upsert_entity(
            state.conn,
            file_id=path.stem,
            path=path,
            mtime_ns=path.stat().st_mtime_ns,
            entity=result.entity,
            body=result.body,
        )
    return {"ok": True}
