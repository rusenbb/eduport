# Desktop Packaging

Eduport packages the Python FastAPI sidecar as a Tauri external binary.
The packaged desktop app does not require `eduport-sidecar` to be installed on
`PATH`.

## Local Build

From the repository root:

```bash
npm ci --prefix frontend
uv run --project sidecar pytest -q
npm --prefix frontend run check
python3 scripts/build_desktop.py
```

On Linux, `build_desktop.py` builds `deb` and `rpm` bundles by default. To
request a different bundle set, pass `TAURI_BUNDLES`, for example:

```bash
TAURI_BUNDLES=appimage python3 scripts/build_desktop.py
```

Bundles are written under:

```text
target/release/bundle/
```

(The repo became a Cargo workspace in Phase 4 of the rewrite. The Tauri
crate now lives at `crates/eduport-tauri/`; build artifacts go to the
workspace-root `target/`.)

The Tauri build runs `scripts/build_tauri_prereqs.mjs`, which:

1. builds `sidecar/pyinstaller_entry.py` into
   `crates/eduport-tauri/binaries/eduport-sidecar-<target-triple>`,
2. builds the Svelte frontend into `frontend/build`,
3. lets Tauri bundle both artifacts.

## GitHub Actions

The `Desktop packages` workflow builds Linux, macOS, and Windows packages and
uploads each platform's `target/release/bundle/**` output as an
artifact. The artifacts are unsigned unless signing credentials are added.

## Signing

macOS notarization and Windows signing require project-owned certificates and
GitHub Actions secrets. The repo intentionally does not include placeholder
credentials. Once those accounts/certificates exist, add the secrets to GitHub
Actions and wire them into the platform-specific build steps.
