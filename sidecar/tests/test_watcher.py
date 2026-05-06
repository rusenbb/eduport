import time
from pathlib import Path

import pytest

from eduport.watcher import EduportWatcher


@pytest.fixture
def folder(tmp_path: Path) -> Path:
    return tmp_path


def _wait(predicate, timeout: float = 2.0) -> None:
    end = time.time() + timeout
    while time.time() < end:
        if predicate():
            return
        time.sleep(0.05)
    raise AssertionError("predicate never became true")


def test_watcher_fires_on_create_modify_delete(folder):
    seen: list[tuple[str, str]] = []

    def on_event(kind: str, path: Path) -> None:
        seen.append((kind, path.name))

    watcher = EduportWatcher(folder, on_event)
    watcher.start()
    try:
        f = folder / "x-Y2y2.md"
        f.write_text("hello")
        _wait(lambda: any(k == "created" and n == "x-Y2y2.md" for k, n in seen))

        f.write_text("hello again")
        _wait(lambda: any(k == "modified" and n == "x-Y2y2.md" for k, n in seen))

        f.unlink()
        _wait(lambda: any(k == "deleted" and n == "x-Y2y2.md" for k, n in seen))
    finally:
        watcher.stop()
