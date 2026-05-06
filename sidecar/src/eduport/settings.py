from __future__ import annotations

import tomllib
from pathlib import Path
from typing import Literal, Optional

import tomli_w
from pydantic import BaseModel, ConfigDict, Field

Theme = Literal["system", "light", "dark"]


class Settings(BaseModel):
    model_config = ConfigDict(extra="forbid")

    data_folder: Path
    attachments_folder: str = Field(default="./attachments")
    notes_folder: str = Field(default="./notes")
    theme: Theme = "system"
    user_email: str

    def resolved_attachments_folder(self) -> Path:
        return (self.data_folder / self.attachments_folder).resolve()

    def resolved_notes_folder(self) -> Path:
        return (self.data_folder / self.notes_folder).resolve()


def load_settings(path: Path) -> Optional[Settings]:
    if not path.exists():
        return None
    return Settings.model_validate(tomllib.loads(path.read_text(encoding="utf-8")))


def save_settings(settings: Settings, path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = settings.model_dump(mode="json")
    payload["data_folder"] = str(payload["data_folder"])
    path.write_bytes(tomli_w.dumps(payload).encode("utf-8"))
