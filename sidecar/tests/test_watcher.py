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


def test_watcher_emits_schema_modified_for_schema_yaml(folder):
    """External edits to `.eduport/schema.yaml` produce a `schema_modified`
    event; non-schema files in `.eduport/` are ignored."""
    seen: list[tuple[str, str]] = []

    def on_event(kind: str, path: Path) -> None:
        seen.append((kind, path.name))

    watcher = EduportWatcher(folder, on_event)
    watcher.start()
    try:
        schema_dir = folder / ".eduport"
        schema_path = schema_dir / "schema.yaml"
        schema_path.write_text("version: 1\ntypes: {}\n")
        _wait(
            lambda: any(
                k == "schema_modified" and n == "schema.yaml" for k, n in seen
            )
        )

        # Other files in .eduport/ are ignored by the schema handler.
        seen.clear()
        (schema_dir / "irrelevant.txt").write_text("x")
        # Allow watchdog a moment to NOT fire schema_modified.
        import time as _t

        _t.sleep(0.3)
        assert all(k != "schema_modified" for k, _ in seen)
    finally:
        watcher.stop()
