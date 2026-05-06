from __future__ import annotations

import argparse
import sqlite3
import sys
from pathlib import Path

import platformdirs
import uvicorn

from eduport.api.app import build_app
from eduport.index.schema import init_schema
from eduport.logging_setup import configure_logging
from eduport.settings import load_settings


def _index_path(data_folder: Path) -> Path:
    cache_dir = Path(platformdirs.user_cache_dir("Eduport", appauthor=False))
    cache_dir.mkdir(parents=True, exist_ok=True)
    folder_hash = abs(hash(str(data_folder.resolve()))) % (2**32)
    return cache_dir / f"index-{folder_hash:08x}.sqlite"


def _log_path() -> Path:
    log_dir = Path(platformdirs.user_log_dir("Eduport", appauthor=False))
    log_dir.mkdir(parents=True, exist_ok=True)
    return log_dir / "sidecar.log"


def _settings_path() -> Path:
    cfg = Path(platformdirs.user_config_dir("Eduport", appauthor=False))
    return cfg / "settings.toml"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="eduport-sidecar")
    parser.add_argument("--port", type=int, default=0, help="bind port (0 = random)")
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--settings", type=Path, default=None, help="override settings path")
    args = parser.parse_args(argv)

    configure_logging(_log_path())
    settings_file = args.settings or _settings_path()
    settings = load_settings(settings_file)
    if settings is None:
        sys.stderr.write(
            f"No settings found at {settings_file}. The launcher should write one before starting the sidecar.\n"
        )
        return 2

    conn = sqlite3.connect(_index_path(settings.data_folder), check_same_thread=False)
    init_schema(conn)

    app = build_app(settings=settings, conn=conn, start_watcher=True, run_reconcile=True)
    uvicorn.run(app, host=args.host, port=args.port)
    return 0
