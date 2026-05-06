from __future__ import annotations

import json
from pathlib import Path

import yaml

from eduport.models import BaseEntity


def serialize_entity_to_markdown(entity: BaseEntity, body: str) -> str:
    """Render entity to '---\\nYAML\\n---\\n\\nbody' format."""
    payload = json.loads(entity.model_dump_json(by_alias=True))
    yaml_block = yaml.safe_dump(payload, sort_keys=False, allow_unicode=True).strip()
    body_stripped = body.lstrip("\n")
    return f"---\n{yaml_block}\n---\n\n{body_stripped}".rstrip() + "\n"


class EntityFileStore:
    """Owns writes to .md files. Tracks recent writes to suppress watcher feedback."""

    def __init__(self, data_folder: Path) -> None:
        self.data_folder = data_folder
        self._recent_writes: set[Path] = set()

    def write(self, file_id: str, entity: BaseEntity, body: str) -> Path:
        path = self.data_folder / f"{file_id}.md"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(serialize_entity_to_markdown(entity, body), encoding="utf-8")
        self._recent_writes.add(path.resolve())
        return path

    def delete_marker(self, path: Path) -> None:
        """Mark a path as recently-touched, even though we didn't write to it.

        Used by the soft-delete flow.
        """
        self._recent_writes.add(path.resolve())

    def was_recently_written(self, path: Path) -> bool:
        """Return True (and consume the marker) if `path` was just written by this store."""
        resolved = path.resolve()
        if resolved in self._recent_writes:
            self._recent_writes.discard(resolved)
            return True
        return False
