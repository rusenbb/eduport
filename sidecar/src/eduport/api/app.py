from __future__ import annotations

import sqlite3
from contextlib import asynccontextmanager
from pathlib import Path
from typing import Optional

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from eduport.api.checkbox import router as checkbox_router
from eduport.api.deps import AppState
from eduport.api.eml_import import router as eml_router
from eduport.api.entities import router as entities_router
from eduport.api.health import router as health_router
from eduport.api.metadata_api import router as metadata_router
from eduport.api.properties_api import router as properties_router
from eduport.api.schema_api import router as schema_router
from eduport.api.search import router as search_router
from eduport.api.settings_api import router as settings_router
from eduport.api.status_api import router as status_router
from eduport.api.trash_api import router as trash_router
from eduport.api.views_api import router as views_router
from eduport.index.reconcile import reconcile
from eduport.index.writer import (
    clear_parse_error,
    delete_entity,
    record_parse_error,
    reindex_all_properties,
    upsert_entity,
)
from eduport.parsers.entity import ParseError, parse_file
from eduport.settings import Settings
from eduport.store.files import EntityFileStore
from eduport.store.schema_store import SchemaStore
from eduport.store.trash import LocalTrash
from eduport.store.view_store import ViewStore
from eduport.watcher import EduportWatcher


def _on_schema_changed(state: AppState) -> None:
    """Reload the schema file and re-index every entity's `properties` rows.

    Triggered when `.eduport/schema.yaml` changes on disk — either because
    the user edited it directly in their editor, or because sync delivered
    a remote update.
    """
    state.schema_store.reload()
    reindex_all_properties(state.conn, state.schema_store.current())


def _on_views_changed(state: AppState) -> None:
    """Reload the views file. Cheap — no derived index to rebuild."""
    state.view_store.reload()


def build_app(
    settings: Settings,
    conn: sqlite3.Connection,
    settings_path: Path | None = None,
    start_watcher: bool = True,
    run_reconcile: bool = False,
    rebuild_index_after_init: bool = False,
) -> FastAPI:
    @asynccontextmanager
    async def lifespan(app: FastAPI):
        # When init_schema migrated FTS5, the entities table still has rows
        # but FTS5 is empty — re-derive both FTS5 and properties from
        # cached frontmatter before the watcher / reconcile take over.
        if rebuild_index_after_init:
            reindex_all_properties(
                app.state.eduport.conn,
                app.state.eduport.schema_store.current(),
            )
        if run_reconcile:
            reconcile(
                app.state.eduport.conn,
                settings.data_folder,
                schema=app.state.eduport.schema_store.current(),
            )
        watcher: Optional[EduportWatcher] = None
        if start_watcher:

            def on_event(kind: str, path: Path) -> None:
                state = app.state.eduport
                if kind == "schema_modified":
                    _on_schema_changed(state)
                    return
                if kind == "views_modified":
                    _on_views_changed(state)
                    return
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
                    schema=state.schema_store.current(),
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
    app.add_middleware(
        CORSMiddleware,
        allow_origin_regex=r"^https?://(localhost|127\.0\.0\.1|tauri\.localhost)(:\d+)?$|^tauri://localhost$",
        allow_credentials=False,
        allow_methods=["*"],
        allow_headers=["*"],
    )
    schema_store = SchemaStore(settings.data_folder)
    schema_store.load()  # seeds .eduport/schema.yaml on first run
    view_store = ViewStore(settings.data_folder)
    view_store.load()  # seeds .eduport/views.yaml on first run
    app.state.eduport = AppState(
        settings=settings,
        conn=conn,
        settings_path=settings_path,
        file_store=EntityFileStore(settings.data_folder),
        trash=LocalTrash(settings.data_folder),
        schema_store=schema_store,
        view_store=view_store,
    )
    app.include_router(health_router)
    app.include_router(entities_router)
    app.include_router(search_router)
    app.include_router(metadata_router)
    app.include_router(checkbox_router)
    app.include_router(eml_router)
    app.include_router(properties_router)
    app.include_router(schema_router)
    app.include_router(settings_router)
    app.include_router(status_router)
    app.include_router(trash_router)
    app.include_router(views_router)
    return app
