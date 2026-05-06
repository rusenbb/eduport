from fastapi import APIRouter, Depends

from eduport.api.deps import AppState, get_state

router = APIRouter()


@router.get("/counts")
def counts(state: AppState = Depends(get_state)) -> dict[str, int]:
    rows = state.conn.execute(
        "SELECT type, COUNT(*) FROM entities GROUP BY type ORDER BY type"
    ).fetchall()
    return {type_: count for type_, count in rows}


@router.get("/tags")
def tags(state: AppState = Depends(get_state)) -> list[dict]:
    rows = state.conn.execute(
        """
        SELECT tag, COUNT(*) AS count
        FROM entity_tags
        WHERE tag NOT LIKE 'eduport-type/%'
          AND tag NOT LIKE 'eduport-doctype/%'
        GROUP BY tag
        ORDER BY count DESC, tag
        """
    ).fetchall()
    return [{"tag": tag, "count": count} for tag, count in rows]
