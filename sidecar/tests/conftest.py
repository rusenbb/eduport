import sqlite3
from pathlib import Path

import pytest
from fastapi.testclient import TestClient

from eduport.api.app import build_app
from eduport.index.schema import init_schema
from eduport.settings import Settings


@pytest.fixture
def settings(tmp_path: Path) -> Settings:
    data = tmp_path / "data"
    data.mkdir()
    (data / "attachments").mkdir()
    (data / "notes").mkdir()
    return Settings(
        data_folder=data,
        attachments_folder="./attachments",
        notes_folder="./notes",
        theme="system",
        user_email="me@example.com",
    )


@pytest.fixture
def conn(tmp_path: Path) -> sqlite3.Connection:
    c = sqlite3.connect(tmp_path / "index.db")
    init_schema(c)
    return c


@pytest.fixture
def client(settings: Settings, conn: sqlite3.Connection) -> TestClient:
    app = build_app(settings=settings, conn=conn, start_watcher=False)
    return TestClient(app)
