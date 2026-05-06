from __future__ import annotations

import sqlite3
from contextlib import asynccontextmanager
from pathlib import Path
from typing import Optional

from fastapi import FastAPI

from eduport.api.checkbox import router as checkbox_router
from eduport.api.deps import AppState
from eduport.api.eml_import import router as eml_router
from eduport.api.entities import router as entities_router
from eduport.api.health import router as health_router
from eduport.api.search import router as search_router
from eduport.api.settings_api import router as settings_router
from eduport.index.reconcile import reconcile
from eduport.index.writer import (
    clear_parse_error,
    delete_entity,
    record_parse_error,
    upsert_entity,
)
from eduport.parsers.entity import ParsedEntity, ParseError, parse_file
from eduport.settings import Settings
from eduport.store.files import EntityFileStore
from eduport.store.trash import LocalTrash
from eduport.watcher import EduportWatcher


def build_app(
    settings: Settings,
    conn: sqlite3.Connection,
    start_watcher: bool = True,
    run_reconcile: bool = False,
) -> FastAPI:
    @asynccontextmanager
    async def lifespan(app: FastAPI):
        if run_reconcile:
            reconcile(app.state.eduport.conn, settings.data_folder)
        watcher: Optional[EduportWatcher] = None
        if start_watcher:
            def on_event(kind: str, path: Path) -> None:
                state = app.state.eduport
                if state.file_store.was_recently_written(path):
                    return
                if kind == "deleted":
                    delete_entity(state.conn, path.stem)
                    return
                result = parse_file(path)
                if isinstance(result, ParseError):
                    record_parse_error(state.conn, str(path), result.message)
                    return
                upsert_entity(
                    state.conn,
                    file_id=path.stem,
                    path=path,
                    mtime_ns=path.stat().st_mtime_ns,
                    entity=result.entity,
                    body=result.body,
                )
                clear_parse_error(state.conn, str(path))

            watcher = EduportWatcher(settings.data_folder, on_event)
            watcher.start()
        try:
            yield
        finally:
            if watcher is not None:
                watcher.stop()

    app = FastAPI(title="Eduport sidecar", version="0.1.0", lifespan=lifespan)
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
