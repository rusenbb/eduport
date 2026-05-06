from fastapi import APIRouter, Depends

from eduport.api.deps import AppState, get_state
from eduport.index.reader import search_fts

router = APIRouter()


@router.get("/search")
def search(q: str, limit: int = 50, state: AppState = Depends(get_state)) -> list[dict]:
    if not q.strip():
        return []
    return search_fts(state.conn, q, limit=limit)
