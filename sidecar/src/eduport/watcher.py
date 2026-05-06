from __future__ import annotations

import logging
from pathlib import Path
from typing import Callable, Optional

from watchdog.events import FileSystemEvent, FileSystemEventHandler
from watchdog.observers import Observer
from watchdog.observers.api import BaseObserver

log = logging.getLogger("eduport.watcher")

EventCallback = Callable[[str, Path], None]


class _Handler(FileSystemEventHandler):
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


class EduportWatcher:
    def __init__(self, folder: Path, callback: EventCallback) -> None:
        self.folder = folder
        self.callback = callback
        self._observer: Optional[BaseObserver] = None

    def start(self) -> None:
        if self._observer is not None:
            return
        observer = Observer()
        observer.schedule(_Handler(self.callback), str(self.folder), recursive=False)
        observer.start()
        self._observer = observer
        log.info("watcher started on %s", self.folder)

    def stop(self) -> None:
        if self._observer is None:
            return
        self._observer.stop()
        self._observer.join(timeout=2.0)
        self._observer = None
        log.info("watcher stopped")
