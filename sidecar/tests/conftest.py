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
    # check_same_thread=False allows the FastAPI TestClient (which uses worker threads)
    # to share the connection. Real production use is single-threaded inside the
    # sidecar process, so this also matches how the app will run.
    c = sqlite3.connect(tmp_path / "index.db", check_same_thread=False)
    init_schema(c)
    return c


@pytest.fixture
def client(settings: Settings, conn: sqlite3.Connection) -> TestClient:
    app = build_app(settings=settings, conn=conn, start_watcher=False)
    return TestClient(app)
