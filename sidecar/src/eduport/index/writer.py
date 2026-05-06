from __future__ import annotations

import json
import sqlite3
from pathlib import Path

from eduport.models import BaseEntity
from eduport.parsers.wikilinks import extract_targets, resolve


def resolve_links(conn: sqlite3.Connection) -> None:
    candidates = [row[0] for row in conn.execute("SELECT file_id FROM entities")]
    rows = conn.execute(
        "SELECT src_file_id, field, target FROM entity_links"
    ).fetchall()
    conn.executemany(
        "UPDATE entity_links SET resolved = ? WHERE src_file_id = ? AND field = ? AND target = ?",
        [
            (resolve(target, candidates), src_file_id, field, target)
            for src_file_id, field, target in rows
        ],
    )


def upsert_entity(
    conn: sqlite3.Connection,
    file_id: str,
    path: Path,
    mtime_ns: int,
    entity: BaseEntity,
    body: str,
) -> None:
    fm_json = entity.model_dump_json(by_alias=True)
    cur = conn.cursor()
    cur.execute("BEGIN")
    try:
        cur.execute(
            "INSERT OR REPLACE INTO entities(file_id, type, name, path, mtime_ns, body, frontmatter) "
            "VALUES (?, ?, ?, ?, ?, ?, ?)",
            (
                file_id,
                entity.entity_type().value,
                entity.name,
                str(path),
                mtime_ns,
                body,
                fm_json,
            ),
        )
        rowid = cur.execute(
            "SELECT rowid FROM entities WHERE file_id = ?", (file_id,)
        ).fetchone()[0]

        cur.execute("DELETE FROM entity_tags WHERE file_id = ?", (file_id,))
        cur.executemany(
            "INSERT INTO entity_tags(file_id, tag) VALUES (?, ?)",
            [(file_id, t) for t in entity.tags],
        )

        cur.execute("DELETE FROM entity_links WHERE src_file_id = ?", (file_id,))
        fm_payload = json.loads(fm_json)
        link_rows = []
        candidates = [row[0] for row in cur.execute("SELECT file_id FROM entities")]
        for field, value in fm_payload.items():
            for target in extract_targets(value):
                link_rows.append((file_id, field, target, resolve(target, candidates)))
        if link_rows:
            cur.executemany(
                "INSERT OR IGNORE INTO entity_links(src_file_id, field, target, resolved) "
                "VALUES (?, ?, ?, ?)",
                link_rows,
            )
        resolve_links(conn)

        cur.execute("DELETE FROM entities_fts WHERE rowid = ?", (rowid,))
        cur.execute(
            "INSERT INTO entities_fts(rowid, body, name, tags) VALUES (?, ?, ?, ?)",
            (rowid, body, entity.name, " ".join(entity.tags)),
        )
        conn.commit()
    except Exception:
        conn.rollback()
        raise


def delete_entity(conn: sqlite3.Connection, file_id: str) -> None:
    cur = conn.cursor()
    cur.execute("BEGIN")
    try:
        rowid_row = cur.execute(
            "SELECT rowid FROM entities WHERE file_id = ?", (file_id,)
        ).fetchone()
        if rowid_row is not None:
            cur.execute("DELETE FROM entities_fts WHERE rowid = ?", (rowid_row[0],))
        cur.execute("DELETE FROM entities WHERE file_id = ?", (file_id,))
        cur.execute("DELETE FROM entity_tags WHERE file_id = ?", (file_id,))
        cur.execute("DELETE FROM entity_links WHERE src_file_id = ?", (file_id,))
        resolve_links(conn)
        conn.commit()
    except Exception:
        conn.rollback()
        raise


def record_parse_error(conn: sqlite3.Connection, path: str, message: str) -> None:
    conn.execute(
        "INSERT OR REPLACE INTO parse_errors(path, message) VALUES (?, ?)",
        (path, message),
    )
    conn.commit()


def clear_parse_error(conn: sqlite3.Connection, path: str) -> None:
    conn.execute("DELETE FROM parse_errors WHERE path = ?", (path,))
    conn.commit()
