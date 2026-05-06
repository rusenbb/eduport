from pathlib import Path

from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel

from eduport.api.deps import AppState, get_state

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
    lines = text.splitlines()
    body_start = 0
    if lines and lines[0] == "---":
        for i in range(1, len(lines)):
            if lines[i] == "---":
                body_start = i + 1
                if body_start < len(lines) and lines[body_start] == "":
                    body_start += 1
                break
    target = body_start + payload.line
    if target >= len(lines):
        raise HTTPException(status_code=400, detail="line out of range")
    line = lines[target]
    new_marker = "[x]" if payload.checked else "[ ]"
    if line.startswith("- [ ]"):
        lines[target] = "- " + new_marker + line[len("- [ ]"):]
    elif line.startswith("- [x]") or line.startswith("- [X]"):
        lines[target] = "- " + new_marker + line[len("- [x]"):]
    else:
        raise HTTPException(status_code=400, detail="line is not a checkbox")
    new_text = "\n".join(lines) + ("\n" if text.endswith("\n") else "")
    path.write_text(new_text, encoding="utf-8")
    state.file_store.delete_marker(path)
    return {"ok": True}
