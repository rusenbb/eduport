from typing import Annotated, Optional

from fastapi import APIRouter, Depends, Query

from eduport.api.deps import AppState, get_state
from eduport.index.reader import search_fts

router = APIRouter()


@router.get("/search")
def search(
    q: str,
    limit: int = 50,
    tag: Annotated[Optional[list[str]], Query()] = None,
    state: AppState = Depends(get_state),
) -> list[dict]:
    if not q.strip():
        return []
    return search_fts(state.conn, q, limit=limit, tags=tag or [])
