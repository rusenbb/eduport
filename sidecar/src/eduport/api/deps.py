from __future__ import annotations

import sqlite3
from dataclasses import dataclass

from fastapi import Request

from eduport.settings import Settings
from eduport.store.files import EntityFileStore
from eduport.store.trash import LocalTrash


@dataclass
class AppState:
    settings: Settings
    conn: sqlite3.Connection
    file_store: EntityFileStore
    trash: LocalTrash


def get_state(request: Request) -> AppState:
    return request.app.state.eduport
