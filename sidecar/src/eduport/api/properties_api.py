"""HTTP API for custom-property aggregations and filter/sort queries.

Endpoints:
    GET /api/properties/counts/{type}/{key}   — sidebar chip aggregations
    GET /api/properties/filter/{type}         — filtered + sorted entity list

Filter syntax (query string):
    text=key:value             — equality on value_text (single-select / url / text / relation)
    num=key:lo..hi             — range on value_num (open with empty string: ``key:..5``)
    date=key:lo..hi            — range on value_date (ISO yyyy-mm-dd)
    sort=key                   — sort by property
    sort_dir=asc|desc          — direction (default asc)

Multiple text/num/date filters AND together. Examples::

    /api/properties/filter/university?text=tier:reach&sort=rank&sort_dir=desc
    /api/properties/filter/university?num=rank:1..10&date=deadline:2026-01-01..
"""

from __future__ import annotations

from typing import Annotated, Optional

from fastapi import APIRouter, Depends, Query

from eduport.api.deps import AppState, get_state
from eduport.index.reader import (
    filter_entities_by_properties,
    property_value_counts,
)
from eduport.models import EntityType

router = APIRouter(prefix="/api/properties")


def _parse_kv(s: str) -> tuple[str, str]:
    """Split a single ``key:value`` filter string. Raises ValueError on bad shape."""
    if ":" not in s:
        raise ValueError(f"expected 'key:value', got {s!r}")
    key, _, val = s.partition(":")
    return key, val


def _parse_range(s: str) -> tuple[Optional[str], Optional[str]]:
    """Parse ``lo..hi``. Either side may be empty (open-ended)."""
    if ".." not in s:
        # treat as exact equality lo == hi
        return s, s
    lo, _, hi = s.partition("..")
    return (lo or None, hi or None)


@router.get("/counts/{entity_type}/{key}")
def counts(
    entity_type: EntityType,
    key: str,
    state: AppState = Depends(get_state),
) -> dict:
    rows = property_value_counts(state.conn, entity_type.value, key)
    return {"entity_type": entity_type.value, "key": key, "values": rows}


@router.get("/filter/{entity_type}")
def filter_(
    entity_type: EntityType,
    text: Annotated[Optional[list[str]], Query()] = None,
    num: Annotated[Optional[list[str]], Query()] = None,
    date: Annotated[Optional[list[str]], Query()] = None,
    sort: Optional[str] = None,
    sort_dir: str = "asc",
    state: AppState = Depends(get_state),
) -> list[dict]:
    text_filters: dict[str, str] = {}
    for s in text or []:
        k, v = _parse_kv(s)
        text_filters[k] = v

    num_filters: dict[str, tuple[Optional[float], Optional[float]]] = {}
    for s in num or []:
        k, v = _parse_kv(s)
        lo_s, hi_s = _parse_range(v)
        num_filters[k] = (
            float(lo_s) if lo_s is not None else None,
            float(hi_s) if hi_s is not None else None,
        )

    date_filters: dict[str, tuple[Optional[str], Optional[str]]] = {}
    for s in date or []:
        k, v = _parse_kv(s)
        lo_s, hi_s = _parse_range(v)
        date_filters[k] = (lo_s, hi_s)

    return filter_entities_by_properties(
        state.conn,
        entity_type.value,
        text_filters=text_filters,
        num_range_filters=num_filters,
        date_range_filters=date_filters,
        sort_key=sort,
        sort_dir=sort_dir,
    )
