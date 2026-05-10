#!/usr/bin/env python3
"""Pre-build hook for `cargo tauri build`.

Used to also build the Python sidecar — that step was removed when
the Rust eduport-core landed (rewrite phases 4–11). The script
stays so `tauri.conf.json:beforeBuildCommand` keeps working; it
now only rebuilds the SvelteKit frontend bundle that Tauri
embeds.
"""
from __future__ import annotations

import shutil
import subprocess
from pathlib import Path


REPO = Path(__file__).resolve().parents[1]
# On Windows npm is npm.cmd; subprocess won't honor PATHEXT, so resolve via
# shutil.which (which DOES honor PATHEXT). Falls back to the literal name on
# *nix and as a clearer error message if npm isn't on PATH at all.
NPM = shutil.which("npm") or "npm"


def run(command: list[str], cwd: Path = REPO) -> None:
    print("+", " ".join(command), flush=True)
    subprocess.run(command, cwd=cwd, check=True)


def main() -> int:
    run([NPM, "--prefix", "frontend", "run", "build"])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
