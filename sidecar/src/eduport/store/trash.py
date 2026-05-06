from __future__ import annotations

import logging
from pathlib import Path
from typing import Optional

import send2trash

log = logging.getLogger("eduport.trash")


class LocalTrash:
    """Move-to-trash with a per-data-folder `.eduport-trash/` subdirectory.

    Each trashed file's original parent path is encoded in a sidecar metadata
    file so we can restore later.
    """

    def __init__(self, data_folder: Path) -> None:
        self.data_folder = data_folder
        self.trash_dir = data_folder / ".eduport-trash"

    def trash(self, path: Path) -> Path:
        self.trash_dir.mkdir(parents=True, exist_ok=True)
        target = self._unique_destination(path.name)
        path.rename(target)
        meta = target.with_suffix(target.suffix + ".restore-from")
        meta.write_text(str(path), encoding="utf-8")
        return target

    def restore(self, trashed: Path) -> Path:
        meta = trashed.with_suffix(trashed.suffix + ".restore-from")
        if not meta.exists():
            raise FileNotFoundError(f"no restore metadata for {trashed}")
        original = Path(meta.read_text(encoding="utf-8"))
        original.parent.mkdir(parents=True, exist_ok=True)
        trashed.rename(original)
        meta.unlink()
        return original

    def _unique_destination(self, name: str) -> Path:
        candidate = self.trash_dir / name
        if not candidate.exists():
            return candidate
        stem, suffix = candidate.stem, candidate.suffix
        i = 1
        while True:
            alt = self.trash_dir / f"{stem}.{i}{suffix}"
            if not alt.exists():
                return alt
            i += 1


def trash_with_fallback(path: Path, fallback: LocalTrash) -> Optional[Path]:
    """Move to OS trash via send2trash; fall back to LocalTrash on error.

    Returns the path inside the local fallback if used, None when OS trash succeeded.
    """
    try:
        send2trash.send2trash(str(path))
        return None
    except OSError as exc:
        log.warning("OS trash failed for %s: %s — falling back to local", path, exc)
        return fallback.trash(path)
