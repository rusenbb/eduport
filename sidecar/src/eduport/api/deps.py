from __future__ import annotations

import sqlite3
from dataclasses import dataclass
from pathlib import Path

from fastapi import Request

from eduport.settings import Settings
from eduport.store.files import EntityFileStore
from eduport.store.schema_store import SchemaStore
from eduport.store.trash import LocalTrash
from eduport.store.view_store import ViewStore


@dataclass
class AppState:
    settings: Settings
    conn: sqlite3.Connection
    settings_path: Path | None
    file_store: EntityFileStore
    trash: LocalTrash
    schema_store: SchemaStore
    view_store: ViewStore


def get_state(request: Request) -> AppState:
    return request.app.state.eduport
