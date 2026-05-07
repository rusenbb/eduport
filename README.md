# Eduport

**English** · [Türkçe](README.tr.md)

<img src="src-tauri/icons/icon.svg" alt="Eduport" width="80" align="right" />

A single-user desktop app for tracking university applications, programs, people, labs, and the documents and emails that connect them. Storage is plain Markdown + YAML on disk — sync-friendly via Dropbox / iCloud / Syncthing, and editable directly in Obsidian alongside the app.

**Status:** v1 in active development. Local builds work end-to-end; signed installers and CI distribution are deferred.

## Highlights

- **8 entity types** — University, Lab, Person, Program, Application, Document, Email, Note — all stored as `<slug>-<id>.md` with YAML frontmatter
- **Wikilinks as the relationship graph** — `[[eth-zurich-K9p3]]` references resolve by id-suffix, so renames in Obsidian don't break links
- **Three-pane UI** — sidebar nav (with counts and tag chips), list/kanban toggle, detail panel with structured fields + rendered body
- **Application kanban** — drag cards across status columns; in-app inline checkbox toggling on bodies
- **⌘K command palette** — quick lookup + full-text search via SQLite FTS5
- **First-run onboarding** — picks a data folder, creates `attachments/` and `notes/` subfolders, optional sample seeds
- **Soft delete** — items move to `<data folder>/.eduport-trash/`, restorable from the in-app Trash view
- **Source of truth = your files** — SQLite lives outside the data folder (in the OS cache dir) and rebuilds itself if missing or stale

## Architecture

```
┌─────────────────────────────────────────┐
│ Tauri shell (Rust)                      │
│  • spawns + supervises Python sidecar    │
│  • hosts WebView at the sidecar's URL    │
│  • bridges native dialogs + reveal       │
└─────────────┬───────────────────────────┘
              │ HTTP loopback (127.0.0.1:<random port>)
              ▼
┌─────────────────────────────────────────┐
│ Python sidecar (FastAPI + uvicorn)      │
│  • REST API over .md entity files        │
│  • watchdog file watcher                 │
│  • markdown-it-py + YAML parsing         │
│  • SQLite + FTS5 indexer                 │
└─────────────────────────────────────────┘
```

## Tech stack

| Layer | Choice |
|---|---|
| Native shell | Tauri 2 (Rust) |
| Frontend | SvelteKit + Svelte 5, Tailwind CSS v4, CodeMirror 6 |
| Markdown render (UI) | `marked` + custom wikilink/checkbox extraction |
| Sidecar API | FastAPI + uvicorn, Pydantic v2 |
| File watcher | `watchdog` |
| Storage / search | stdlib `sqlite3` with FTS5 |
| Desktop bundling | Tauri externalBin + PyInstaller for the sidecar |
| Tooling | `uv` (Python), `npm` (frontend), `cargo` (Rust) |

## Project structure

```
docs/                design spec, packaging notes, implementation plans
frontend/            SvelteKit app — UI + API client
scripts/             build helpers (sidecar bundling, Tauri prereqs)
sidecar/             Python (FastAPI) — uv-managed project
src-tauri/           Rust shell (Tauri 2) — entry point + native bridges
```

## Prerequisites

The project targets macOS, Windows, and Linux. All three need:

- **Rust** 1.77.2+ (`rustup install stable`)
- **Node.js** 20+ and npm
- **Python** 3.12+ and [`uv`](https://docs.astral.sh/uv/)

Plus the OS-specific Tauri prerequisites (full reference: <https://v2.tauri.app/start/prerequisites/>):

- **macOS:** Xcode Command Line Tools — `xcode-select --install`
- **Windows:** [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (with the "Desktop development with C++" workload). WebView2 is preinstalled on Windows 10/11.
- **Linux (Debian/Ubuntu):** `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `librsvg2-dev`, `build-essential`

## Getting started

### Run the sidecar standalone (API on an arbitrary port)

```bash
cd sidecar
uv sync
uv run eduport-sidecar
```

### Run the frontend in a browser (UI only, no Tauri bridges)

```bash
cd frontend
npm install
npm run dev          # http://localhost:5173
```

In browser-only mode, dialogs that require Tauri (folder pickers, native file reveal) fall back to dev placeholders.

### Build + install the full desktop app

The fastest path to a real installable app is the packaging script — it bundles the Python sidecar with PyInstaller, builds the SvelteKit frontend, and runs `tauri build`:

```bash
python3 scripts/build_desktop.py
```

On Linux this produces `.deb` and `.rpm` under `src-tauri/target/release/bundle/`. Install with `sudo apt install ./src-tauri/target/release/bundle/deb/eduport_*.deb` (or your distro's equivalent). See [docs/packaging.md](docs/packaging.md) for advanced bundle targets and CI notes.

## Tests + checks

```bash
# Sidecar (Python) — pytest + ruff
cd sidecar
uv run pytest -q
uv run ruff check src/ tests/

# Frontend — type-check + Svelte diagnostics
cd frontend
npm run check
```

## Documentation

- **[Design spec](docs/superpowers/specs/2026-05-06-eduport-design.md)** — entity model, storage layout, UI shape, sync semantics, parse-error handling — the canonical "why" and "what"
- **[Packaging](docs/packaging.md)** — local builds, GitHub Actions workflow, signing notes
- **Implementation plans** — granular per-layer breakdowns under [`docs/superpowers/plans/`](docs/superpowers/plans/)

## License

MIT (per `src-tauri/Cargo.toml`). A standalone `LICENSE` file is on the to-do list.
