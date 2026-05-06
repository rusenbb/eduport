from fastapi import APIRouter, Depends

from eduport.api.deps import AppState, get_state

router = APIRouter()


@router.get("/status")
def status(state: AppState = Depends(get_state)) -> dict:
    parse_errors = state.conn.execute("SELECT COUNT(*) FROM parse_errors").fetchone()[0]
    entities = state.conn.execute("SELECT COUNT(*) FROM entities").fetchone()[0]
    return {"status": "ok", "parse_errors": parse_errors, "entities": entities}


@router.get("/parse-errors")
def parse_errors(state: AppState = Depends(get_state)) -> list[dict]:
    rows = state.conn.execute(
        "SELECT path, message, occurred_at FROM parse_errors ORDER BY occurred_at DESC, path"
    ).fetchall()
    return [
        {"path": path, "message": message, "occurred_at": occurred_at}
        for path, message, occurred_at in rows
    ]
