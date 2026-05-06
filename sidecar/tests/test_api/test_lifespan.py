import sqlite3
from pathlib import Path

from fastapi.testclient import TestClient

from eduport.api.app import build_app
from eduport.index.schema import init_schema
from eduport.settings import Settings


def test_lifespan_runs_initial_reconcile(tmp_path: Path):
    data = tmp_path / "data"
    data.mkdir()
    (data / "attachments").mkdir()
    (data / "notes").mkdir()
    (data / "eth-K9p3.md").write_text("""---
tags: [eduport-type/university]
name: ETH
country: CH
---
""", encoding="utf-8")
    settings = Settings(
        data_folder=data,
        attachments_folder="./attachments",
        notes_folder="./notes",
        theme="system",
        user_email="me@example.com",
    )
    conn = sqlite3.connect(":memory:", check_same_thread=False)
    init_schema(conn)
    app = build_app(settings=settings, conn=conn, start_watcher=False, run_reconcile=True)
    with TestClient(app) as c:
        resp = c.get("/entities/university")
        assert resp.status_code == 200
        names = [r["name"] for r in resp.json()]
        assert "ETH" in names
