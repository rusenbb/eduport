from __future__ import annotations

import sqlite3
from typing import Optional, Sequence


def _row_to_dict(row: sqlite3.Row) -> dict:
    return {k: row[k] for k in row.keys()}


def list_entities(
    conn: sqlite3.Connection,
    type: Optional[str] = None,
    tags: Optional[Sequence[str]] = None,
) -> list[dict]:
    conn.row_factory = sqlite3.Row
    where: list[str] = []
    params: list[object] = []
    if type is not None:
        where.append("type = ?")
        params.append(type)
    if tags:
        placeholders = ", ".join("?" * len(tags))
        where.append(
            f"file_id IN ("
            f"  SELECT file_id FROM entity_tags "
            f"  WHERE tag IN ({placeholders}) "
            f"  GROUP BY file_id HAVING COUNT(DISTINCT tag) = ?"
            f")"
        )
        params.extend(tags)
        params.append(len(tags))
    sql = "SELECT file_id, type, name, path, mtime_ns FROM entities"
    if where:
        sql += " WHERE " + " AND ".join(where)
    sql += " ORDER BY name"
    return [_row_to_dict(row) for row in conn.execute(sql, params)]


def search_fts(
    conn: sqlite3.Connection,
    query: str,
    limit: int = 50,
    tags: Optional[Sequence[str]] = None,
) -> list[dict]:
    conn.row_factory = sqlite3.Row
    params: list[object] = [query]
    tag_clause = ""
    if tags:
        placeholders = ", ".join("?" * len(tags))
        tag_clause = (
            f" AND e.file_id IN ("
            f"  SELECT file_id FROM entity_tags "
            f"  WHERE tag IN ({placeholders}) "
            f"  GROUP BY file_id HAVING COUNT(DISTINCT tag) = ?"
            f")"
        )
        params.extend(tags)
        params.append(len(tags))
    params.append(limit)
    sql = """
        SELECT e.file_id, e.type, e.name,
               snippet(entities_fts, 0, '<<', '>>', '...', 16) AS snippet
        FROM entities_fts
        JOIN entities e ON e.rowid = entities_fts.rowid
        WHERE entities_fts MATCH ?
    """ + tag_clause + """
        LIMIT ?
    """
    return [_row_to_dict(row) for row in conn.execute(sql, params)]


def backlinks(conn: sqlite3.Connection, file_id: str) -> list[dict]:
    conn.row_factory = sqlite3.Row
    sql = """
        SELECT l.src_file_id, l.field, e.type, e.name
        FROM entity_links l
        JOIN entities e ON e.file_id = l.src_file_id
        WHERE resolved = ?
        ORDER BY e.name
    """
    return [_row_to_dict(row) for row in conn.execute(sql, (file_id,))]


def property_value_counts(
    conn: sqlite3.Connection, entity_type: str, key: str
) -> list[dict]:
    """Count entities of ``entity_type`` grouped by ``properties.value_text``
    (or ``value_num`` for checkboxes). Used for sidebar chip aggregation
    on single-select / multi-select / checkbox properties.
    """
    conn.row_factory = sqlite3.Row
    # multi-select rows are JSON arrays in value_multi — we expand them
    # client-side; here we return single-select / relation / url / text
    # via value_text and checkbox via value_num.
    rows = conn.execute(
        """
        SELECT p.type, p.value_text, p.value_num, p.value_multi, COUNT(*) AS count
        FROM properties p
        JOIN entities e ON e.file_id = p.file_id
        WHERE e.type = ? AND p.key = ?
        GROUP BY p.type, p.value_text, p.value_num, p.value_multi
        """,
        (entity_type, key),
    ).fetchall()
    out: dict[tuple[str, str], int] = {}
    for row in rows:
        if row["type"] == "checkbox":
            v = "true" if row["value_num"] == 1.0 else "false"
            out[("checkbox", v)] = out.get(("checkbox", v), 0) + row["count"]
        elif row["type"] == "multi-select" and row["value_multi"]:
            import json as _json

            try:
                values = _json.loads(row["value_multi"])
            except _json.JSONDecodeError:
                continue
            for v in values:
                out[("multi-select", v)] = out.get(("multi-select", v), 0) + row["count"]
        elif row["value_text"] is not None:
            out[(row["type"], row["value_text"])] = (
                out.get((row["type"], row["value_text"]), 0) + row["count"]
            )
    return [
        {"type": t, "value": v, "count": c}
        for (t, v), c in sorted(out.items(), key=lambda x: (-x[1], x[0]))
    ]


def filter_entities_by_properties(
    conn: sqlite3.Connection,
    entity_type: str,
    *,
    text_filters: Optional[dict[str, str]] = None,
    num_range_filters: Optional[dict[str, tuple[Optional[float], Optional[float]]]] = None,
    date_range_filters: Optional[dict[str, tuple[Optional[str], Optional[str]]]] = None,
    sort_key: Optional[str] = None,
    sort_dir: str = "asc",
) -> list[dict]:
    """Return entities of ``entity_type`` that match all property filters,
    optionally sorted by a property value.

    ``text_filters``: ``key → required value`` for value_text equality
    (useful for single-select / url / relation / text exact match).
    ``num_range_filters`` and ``date_range_filters``: ``key → (lo, hi)``;
    None on either side means open-ended.
    """
    conn.row_factory = sqlite3.Row
    join_binds: list[object] = []  # bound to ?-marks in JOIN clauses
    where_binds: list[object] = []  # bound to ?-marks in WHERE clauses
    where = ["e.type = ?"]
    where_binds.append(entity_type)
    joins: list[str] = []

    def _join(alias: str, key: str) -> str:
        join_binds.append(key)
        return f"JOIN properties {alias} ON {alias}.file_id = e.file_id AND {alias}.key = ?"

    text_filters = text_filters or {}
    for i, (key, value) in enumerate(text_filters.items()):
        alias = f"pt{i}"
        joins.append(_join(alias, key))
        where.append(f"{alias}.value_text = ?")
        where_binds.append(value)

    num_range_filters = num_range_filters or {}
    for i, (key, (lo, hi)) in enumerate(num_range_filters.items()):
        alias = f"pn{i}"
        joins.append(_join(alias, key))
        if lo is not None:
            where.append(f"{alias}.value_num >= ?")
            where_binds.append(lo)
        if hi is not None:
            where.append(f"{alias}.value_num <= ?")
            where_binds.append(hi)

    date_range_filters = date_range_filters or {}
    for i, (key, (lo, hi)) in enumerate(date_range_filters.items()):
        alias = f"pd{i}"
        joins.append(_join(alias, key))
        if lo is not None:
            where.append(f"{alias}.value_date >= ?")
            where_binds.append(lo)
        if hi is not None:
            where.append(f"{alias}.value_date <= ?")
            where_binds.append(hi)

    sql = "SELECT e.file_id, e.type, e.name FROM entities e "
    if joins:
        sql += " ".join(joins) + " "
    sort_col: Optional[str] = None
    if sort_key is not None:
        sql += "LEFT JOIN properties ps ON ps.file_id = e.file_id AND ps.key = ? "
        join_binds.append(sort_key)
        sort_col = "COALESCE(ps.value_num, ps.value_date, ps.value_text)"
    sql += "WHERE " + " AND ".join(where)
    if sort_key is not None:
        direction = "DESC" if sort_dir.lower() == "desc" else "ASC"
        sql += f" ORDER BY {sort_col} {direction}, e.name ASC"
    else:
        sql += " ORDER BY e.name"
    return [_row_to_dict(row) for row in conn.execute(sql, join_binds + where_binds)]
