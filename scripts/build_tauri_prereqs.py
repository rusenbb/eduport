#!/usr/bin/env python3
from __future__ import annotations

import shutil
import subprocess
import sys
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
    run([sys.executable, "scripts/build_sidecar.py"])
    run([NPM, "--prefix", "frontend", "run", "build"])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
