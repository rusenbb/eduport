from __future__ import annotations

import json
import sqlite3
from datetime import date
from pathlib import Path
from typing import Any, Optional

from eduport.models import BaseEntity, Schema
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


def _coerce_property_columns(
    prop_type: str, value: Any
) -> tuple[Optional[str], Optional[float], Optional[str], Optional[str]]:
    """Pick the right ``properties`` table column for a custom-field value.

    Returns ``(value_text, value_num, value_date, value_multi)``. Exactly
    one is non-null for known shapes; all-null is returned when the value
    fails to match the property type (the value-warnings layer surfaces
    that case to the user separately).
    """
    text: Optional[str] = None
    num: Optional[float] = None
    iso: Optional[str] = None
    multi: Optional[str] = None

    if prop_type in ("text", "url", "single-select"):
        if isinstance(value, str):
            text = value
    elif prop_type == "relation":
        # store the raw wikilink string; resolved targets live in entity_links
        if isinstance(value, str):
            text = value
    elif prop_type == "number":
        if isinstance(value, bool):
            pass  # bool is wrong type, skip
        elif isinstance(value, (int, float)):
            num = float(value)
    elif prop_type == "checkbox":
        if isinstance(value, bool):
            num = 1.0 if value else 0.0
    elif prop_type == "date":
        if isinstance(value, date):
            iso = value.isoformat()
        elif isinstance(value, str):
            try:
                date.fromisoformat(value)
            except ValueError:
                pass
            else:
                iso = value
    elif prop_type == "multi-select":
        if isinstance(value, list) and all(isinstance(v, str) for v in value):
            multi = json.dumps(value)
    return text, num, iso, multi


def _custom_text_for_fts5(entity: BaseEntity, schema: Schema) -> str:
    """Concatenate text/url custom-property values into a single FTS5 column.

    Used by ``upsert_entity`` to keep the search index aware of custom-field
    content. Only string-shaped values for ``text`` and ``url`` properties
    are included — other types don't carry searchable prose.
    """
    extras: dict[str, Any] = dict(entity.model_extra or {})
    if not extras:
        return ""
    type_schema = schema.for_type(entity.entity_type())
    declared = {p.key: p for p in type_schema.properties}
    parts: list[str] = []
    for key, value in extras.items():
        prop = declared.get(key)
        if prop is None:
            continue
        if prop.type in ("text", "url") and isinstance(value, str):
            parts.append(value)
    return " ".join(parts)


def _upsert_properties(
    cur: sqlite3.Cursor,
    file_id: str,
    entity: BaseEntity,
    schema: Schema,
) -> None:
    """Replace ``properties`` rows for ``file_id`` based on ``entity``'s extras
    cross-referenced against the schema. Skips orphaned and bad-typed values
    (they remain visible in the YAML file and the value-warnings response —
    the SQL index is for valid values only)."""
    cur.execute("DELETE FROM properties WHERE file_id = ?", (file_id,))
    extras: dict[str, Any] = dict(entity.model_extra or {})
    if not extras:
        return
    type_schema = schema.for_type(entity.entity_type())
    declared = {p.key: p for p in type_schema.properties}
    rows: list[tuple] = []
    for key, value in extras.items():
        prop = declared.get(key)
        if prop is None:
            continue  # orphaned — skip the index but keep the YAML
        text, num, iso, multi = _coerce_property_columns(prop.type, value)
        if text is None and num is None and iso is None and multi is None:
            continue  # type-mismatched — skip the index
        rows.append((file_id, key, prop.type, text, num, iso, multi))
    if rows:
        cur.executemany(
            "INSERT INTO properties(file_id, key, type, value_text, value_num, value_date, value_multi) "
            "VALUES (?, ?, ?, ?, ?, ?, ?)",
            rows,
        )


def upsert_entity(
    conn: sqlite3.Connection,
    file_id: str,
    path: Path,
    mtime_ns: int,
    entity: BaseEntity,
    body: str,
    schema: Optional[Schema] = None,
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
        custom_text = _custom_text_for_fts5(entity, schema) if schema is not None else ""
        cur.execute(
            "INSERT INTO entities_fts(rowid, body, name, tags, custom_text) "
            "VALUES (?, ?, ?, ?, ?)",
            (rowid, body, entity.name, " ".join(entity.tags), custom_text),
        )
        if schema is not None:
            _upsert_properties(cur, file_id, entity, schema)
        conn.commit()
    except Exception:
        conn.rollback()
        raise


def reindex_all_properties(conn: sqlite3.Connection, schema: Schema) -> int:
    """Re-derive the ``properties`` table for every entity using ``schema``.

    Called after a schema mutation (add/patch/delete property) so the
    SQL filter/sort surface stays in sync with the schema. Returns the
    number of entities re-indexed.
    """
    from eduport.parsers.entity import _TYPE_TO_MODEL

    rows = conn.execute(
        "SELECT file_id, type, path, mtime_ns, body, frontmatter FROM entities"
    ).fetchall()
    n = 0
    for file_id, type_, path_str, mtime_ns, body, fm_json in rows:
        from eduport.models import EntityType

        try:
            model_cls = _TYPE_TO_MODEL[EntityType(type_)]
            entity = model_cls.model_validate(json.loads(fm_json))
        except Exception:
            continue
        upsert_entity(
            conn,
            file_id=file_id,
            path=Path(path_str),
            mtime_ns=mtime_ns,
            entity=entity,
            body=body,
            schema=schema,
        )
        n += 1
    return n


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
