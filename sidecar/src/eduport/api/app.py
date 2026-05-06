from __future__ import annotations

import sqlite3

from fastapi import FastAPI

from eduport.api.deps import AppState
from eduport.api.health import router as health_router
from eduport.settings import Settings
from eduport.store.files import EntityFileStore
from eduport.store.trash import LocalTrash


def build_app(
    settings: Settings,
    conn: sqlite3.Connection,
    start_watcher: bool = True,
    run_reconcile: bool = False,
) -> FastAPI:
    app = FastAPI(title="Eduport sidecar", version="0.1.0")
    app.state.eduport = AppState(
        settings=settings,
        conn=conn,
        file_store=EntityFileStore(settings.data_folder),
        trash=LocalTrash(settings.data_folder),
    )
    app.include_router(health_router)
    return app
