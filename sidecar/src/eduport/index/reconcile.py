from __future__ import annotations

import logging
import sqlite3
from dataclasses import dataclass
from pathlib import Path
from typing import Optional

from eduport.index.writer import (
    clear_parse_error,
    delete_entity,
    record_parse_error,
    upsert_entity,
)
from eduport.models import Schema
from eduport.parsers.entity import ParseError, parse_file

log = logging.getLogger("eduport.reconcile")


@dataclass
class ReconcileSummary:
    added: int = 0
    updated: int = 0
    removed: int = 0
    unchanged: int = 0
    errors: int = 0


def reconcile(
    conn: sqlite3.Connection,
    data_folder: Path,
    schema: Optional[Schema] = None,
) -> ReconcileSummary:
    summary = ReconcileSummary()

    existing: dict[str, int] = {
        row[0]: row[1]
        for row in conn.execute("SELECT file_id, mtime_ns FROM entities")
    }

    seen_ids: set[str] = set()
    for path in sorted(data_folder.glob("*.md")):
        if path.name.startswith("."):
            continue
        file_id = path.stem
        seen_ids.add(file_id)
        try:
            mtime_ns = path.stat().st_mtime_ns
        except OSError as exc:
            log.warning("stat failed for %s: %s", path, exc)
            continue

        if existing.get(file_id) == mtime_ns:
            summary.unchanged += 1
            continue

        result = parse_file(path)
        if isinstance(result, ParseError):
            record_parse_error(conn, str(path), result.message)
            summary.errors += 1
            continue

        upsert_entity(
            conn,
            file_id=file_id,
            path=result.path,
            mtime_ns=mtime_ns,
            entity=result.entity,
            body=result.body,
            schema=schema,
        )
        clear_parse_error(conn, str(path))
        if file_id in existing:
            summary.updated += 1
        else:
            summary.added += 1

    for file_id in set(existing) - seen_ids:
        delete_entity(conn, file_id)
        summary.removed += 1

    log.info("reconcile: %s", summary)
    return summary
