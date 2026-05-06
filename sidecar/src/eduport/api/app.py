from __future__ import annotations

import sqlite3

from fastapi import FastAPI

from eduport.api.checkbox import router as checkbox_router
from eduport.api.deps import AppState
from eduport.api.eml_import import router as eml_router
from eduport.api.entities import router as entities_router
from eduport.api.health import router as health_router
from eduport.api.search import router as search_router
from eduport.api.settings_api import router as settings_router
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
    app.include_router(entities_router)
    app.include_router(search_router)
    app.include_router(checkbox_router)
    app.include_router(eml_router)
    app.include_router(settings_router)
    return app
