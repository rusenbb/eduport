from __future__ import annotations

import logging
from pathlib import Path
from typing import Callable, Optional

from watchdog.events import FileSystemEvent, FileSystemEventHandler
from watchdog.observers import Observer
from watchdog.observers.api import BaseObserver

from eduport.store.schema_store import SCHEMA_DIR_NAME, SCHEMA_FILENAME
from eduport.store.view_store import VIEWS_FILENAME

log = logging.getLogger("eduport.watcher")

EventCallback = Callable[[str, Path], None]

# Event kinds emitted to callbacks:
#   "created" / "modified" / "deleted"  → entity .md files
#   "schema_modified"                   → .eduport/schema.yaml (any change)
#   "views_modified"                    → .eduport/views.yaml (any change)
SCHEMA_EVENT_KIND = "schema_modified"
VIEWS_EVENT_KIND = "views_modified"


class _EntityHandler(FileSystemEventHandler):
    def __init__(self, callback: EventCallback) -> None:
        self.callback = callback

    def _emit(self, kind: str, raw_path: str) -> None:
        path = Path(raw_path)
        if path.suffix != ".md" or path.name.startswith("."):
            return
        self.callback(kind, path)

    def on_created(self, event: FileSystemEvent) -> None:
        if not event.is_directory:
            self._emit("created", event.src_path)

    def on_modified(self, event: FileSystemEvent) -> None:
        if not event.is_directory:
            self._emit("modified", event.src_path)

    def on_deleted(self, event: FileSystemEvent) -> None:
        if not event.is_directory:
            self._emit("deleted", event.src_path)


class _DotEduportHandler(FileSystemEventHandler):
    """Watches the .eduport/ folder, dispatching by filename to the right
    event kind: schema.yaml → schema_modified, views.yaml → views_modified.
    """

    def __init__(self, callback: EventCallback) -> None:
        self.callback = callback

    def _maybe_emit(self, raw_path: str) -> None:
        path = Path(raw_path)
        if path.name == SCHEMA_FILENAME:
            self.callback(SCHEMA_EVENT_KIND, path)
        elif path.name == VIEWS_FILENAME:
            self.callback(VIEWS_EVENT_KIND, path)

    def on_created(self, event: FileSystemEvent) -> None:
        if not event.is_directory:
            self._maybe_emit(event.src_path)

    def on_modified(self, event: FileSystemEvent) -> None:
        if not event.is_directory:
            self._maybe_emit(event.src_path)

    def on_deleted(self, event: FileSystemEvent) -> None:
        if not event.is_directory:
            self._maybe_emit(event.src_path)


# Backwards-compat alias for any out-of-tree imports (none in this repo,
# but keeps the public symbol stable).
_SchemaHandler = _DotEduportHandler


class EduportWatcher:
    def __init__(self, folder: Path, callback: EventCallback) -> None:
        self.folder = folder
        self.callback = callback
        self._observer: Optional[BaseObserver] = None

    def start(self) -> None:
        if self._observer is not None:
            return
        observer = Observer()
        observer.schedule(_EntityHandler(self.callback), str(self.folder), recursive=False)
        # Schema lives in `.eduport/` — make the dir if missing so the watch
        # can attach. SchemaStore also creates this on first load; this
        # belt-and-suspenders avoids ordering issues.
        schema_dir = self.folder / SCHEMA_DIR_NAME
        schema_dir.mkdir(parents=True, exist_ok=True)
        observer.schedule(_DotEduportHandler(self.callback), str(schema_dir), recursive=False)
        observer.start()
        self._observer = observer
        log.info("watcher started on %s (+ %s)", self.folder, schema_dir)

    def stop(self) -> None:
        if self._observer is None:
            return
        self._observer.stop()
        self._observer.join(timeout=2.0)
        self._observer = None
        log.info("watcher stopped")
