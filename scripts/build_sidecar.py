#!/usr/bin/env python3
from __future__ import annotations

import os
import shutil
import subprocess
import sys
from pathlib import Path


REPO = Path(__file__).resolve().parents[1]
SIDECAR = REPO / "sidecar"
# The repo became a Cargo workspace in Phase 4 of the rewrite; the
# Tauri crate now lives at crates/eduport-tauri/ and the workspace
# target/ is at the repo root.
BIN_DIR = REPO / "crates" / "eduport-tauri" / "binaries"
PYINSTALLER_WORK = REPO / "target" / "pyinstaller-build"
PYINSTALLER_SPEC = REPO / "target" / "pyinstaller-spec"


def run(command: list[str], cwd: Path = REPO) -> None:
    print("+", " ".join(command), flush=True)
    subprocess.run(command, cwd=cwd, check=True)


def host_triple() -> str:
    configured = os.environ.get("TARGET_TRIPLE") or os.environ.get("TAURI_ENV_TARGET_TRIPLE")
    if configured:
        return configured
    output = subprocess.check_output(["rustc", "-vV"], text=True)
    for line in output.splitlines():
        if line.startswith("host: "):
            return line.split(": ", 1)[1].strip()
    raise RuntimeError("could not determine Rust host target triple")


def expected_binary_name(target: str) -> str:
    suffix = ".exe" if "windows" in target else ""
    return f"eduport-sidecar-{target}{suffix}"


def main() -> int:
    if shutil.which("uv") is None:
        raise SystemExit("uv is required to build the sidecar binary")
    if shutil.which("rustc") is None:
        raise SystemExit("rustc is required to determine the target triple")

    target = host_triple()
    name = f"eduport-sidecar-{target}"
    expected = BIN_DIR / expected_binary_name(target)
    BIN_DIR.mkdir(parents=True, exist_ok=True)
    PYINSTALLER_WORK.mkdir(parents=True, exist_ok=True)
    PYINSTALLER_SPEC.mkdir(parents=True, exist_ok=True)

    if expected.exists():
        expected.unlink()

    run(
        [
            "uv",
            "run",
            "--with",
            "pyinstaller",
            "pyinstaller",
            "--noconfirm",
            "--clean",
            "--onefile",
            "--name",
            name,
            "--distpath",
            str(BIN_DIR),
            "--workpath",
            str(PYINSTALLER_WORK),
            "--specpath",
            str(PYINSTALLER_SPEC),
            "pyinstaller_entry.py",
        ],
        cwd=SIDECAR,
    )

    if not expected.exists():
        raise SystemExit(f"expected sidecar binary was not produced: {expected}")

    if not sys.platform.startswith("win"):
        expected.chmod(expected.stat().st_mode | 0o755)

    print(f"Built {expected}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
