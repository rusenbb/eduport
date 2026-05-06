from __future__ import annotations

import tomllib
from pathlib import Path
from typing import Literal, Optional

import tomli_w
from pydantic import BaseModel, ConfigDict, Field, field_validator

Theme = Literal["system", "light", "dark"]


class Settings(BaseModel):
    model_config = ConfigDict(extra="forbid")

    data_folder: Path
    attachments_folder: str = Field(default="./attachments")
    notes_folder: str = Field(default="./notes")
    theme: Theme = "system"
    user_email: str
    zoom_factor: float = Field(default=1.0, ge=0.75, le=1.5)
    obsidian_vault: Optional[str] = None
    confirm_deletes: bool = True

    @field_validator("obsidian_vault")
    @classmethod
    def _blank_obsidian_vault_to_none(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        stripped = value.strip()
        return stripped or None

    def resolved_attachments_folder(self) -> Path:
        return (self.data_folder / self.attachments_folder).resolve()

    def resolved_notes_folder(self) -> Path:
        return (self.data_folder / self.notes_folder).resolve()


def load_settings(path: Path) -> Settings | None:
    if not path.exists():
        return None
    return Settings.model_validate(tomllib.loads(path.read_text(encoding="utf-8")))


def save_settings(settings: Settings, path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = settings.model_dump(mode="json", exclude_none=True)
    path.write_bytes(tomli_w.dumps(payload).encode("utf-8"))
