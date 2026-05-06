#!/usr/bin/env python3
from __future__ import annotations

import subprocess
import sys
from pathlib import Path


REPO = Path(__file__).resolve().parents[1]


def run(command: list[str], cwd: Path = REPO) -> None:
    print("+", " ".join(command), flush=True)
    subprocess.run(command, cwd=cwd, check=True)


def main() -> int:
    run([sys.executable, "scripts/build_sidecar.py"])
    run(["npm", "--prefix", "frontend", "run", "build"])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
