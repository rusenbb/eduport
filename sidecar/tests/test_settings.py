from pathlib import Path

import pytest

from eduport.settings import Settings, load_settings, save_settings


def test_settings_round_trip(tmp_path: Path):
    settings = Settings(
        data_folder=tmp_path / "data",
        attachments_folder="./attachments",
        notes_folder="./notes",
        theme="dark",
        user_email="me@example.com",
    )
    config_file = tmp_path / "settings.toml"
    save_settings(settings, config_file)
    loaded = load_settings(config_file)
    assert loaded == settings


def test_load_missing_returns_none(tmp_path: Path):
    assert load_settings(tmp_path / "missing.toml") is None


def test_resolved_paths(tmp_path: Path):
    settings = Settings(
        data_folder=tmp_path,
        attachments_folder="./attachments",
        notes_folder="./notes",
        theme="system",
        user_email="me@example.com",
    )
    assert settings.resolved_attachments_folder() == tmp_path / "attachments"
    assert settings.resolved_notes_folder() == tmp_path / "notes"


def test_invalid_theme_rejected():
    with pytest.raises(ValueError):
        Settings(
            data_folder=Path("/tmp"),
            attachments_folder="./a",
            notes_folder="./n",
            theme="bogus",  # type: ignore
            user_email="x@x.com",
        )
