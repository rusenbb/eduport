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
) -> list[dict]:
    conn.row_factory = sqlite3.Row
    sql = """
        SELECT e.file_id, e.type, e.name,
               snippet(entities_fts, 0, '<<', '>>', '...', 16) AS snippet
        FROM entities_fts
        JOIN entities e ON e.rowid = entities_fts.rowid
        WHERE entities_fts MATCH ?
        LIMIT ?
    """
    return [_row_to_dict(row) for row in conn.execute(sql, (query, limit))]


def backlinks(conn: sqlite3.Connection, file_id: str) -> list[dict]:
    conn.row_factory = sqlite3.Row
    sql = """
        SELECT src_file_id, field
        FROM entity_links
        WHERE resolved = ?
        ORDER BY src_file_id
    """
    return [_row_to_dict(row) for row in conn.execute(sql, (file_id,))]
