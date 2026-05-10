#!/usr/bin/env python3
from __future__ import annotations

import os
import shutil
import subprocess
import sys
from pathlib import Path


REPO = Path(__file__).resolve().parents[1]
TAURI = REPO / "frontend" / "node_modules" / ".bin" / (
    "tauri.cmd" if os.name == "nt" else "tauri"
)


def run(command: list[str], cwd: Path = REPO) -> None:
    print("+", " ".join(command), flush=True)
    subprocess.run(command, cwd=cwd, check=True)


def main() -> int:
    # 1. Build the SvelteKit frontend bundle that Tauri embeds. We
    #    used to run this through tauri.conf.json's `beforeBuildCommand`,
    #    but the relative path Tauri resolves it from differs across
    #    platforms (macOS runners broke). Running it explicitly here
    #    keeps the cwd predictable.
    npm = shutil.which("npm") or "npm"
    run([npm, "--prefix", "frontend", "run", "build"])

    # 2. Run tauri build.
    command = [str(TAURI), "build"]
    bundles = os.environ.get("TAURI_BUNDLES")
    if bundles:
        command.extend(["--bundles", bundles])
    elif sys.platform.startswith("linux"):
        # AppImage creation depends on linuxdeploy/FUSE details that are brittle in
        # CI and containers. Build the installable native Linux packages by default.
        command.extend(["--bundles", "deb,rpm"])
    run(command)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
