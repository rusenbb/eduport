# Eduport

**English** · [Türkçe](README.tr.md)

<img src="crates/eduport-tauri/icons/icon.svg" alt="Eduport" width="80" align="right" />

A single-user desktop app for tracking university applications, programs, people, labs, and the documents and emails that connect them. Storage is plain Markdown + YAML on disk — sync-friendly via Dropbox / iCloud / Syncthing, and editable directly in Obsidian alongside the app.

**Just want to use it?** Download an installer for Windows, macOS, or Linux from the [latest release](https://github.com/rusenbb/eduport/releases/latest) — the release page lists exactly which file to pick for your OS and how to install it. No setup beyond that.

**Status:** v1 in active development. Local builds work end-to-end; installers are unsigned (warnings on first launch are dismissable, see the release page for steps).

## Highlights

- **8 entity types** — University, Lab, Person, Program, Application, Document, Email, Note — all stored as `<slug>-<id>.md` with YAML frontmatter
- **User-defined custom properties (Notion-style)** — per entity type, eight property types (text / number / date / checkbox / single-select / multi-select / url / relation), defined in a sync-friendly `.eduport/schema.yaml` so renames and new fields propagate across machines
- **Three view modes per entity type** — List, Table (editable cells), and Kanban (Application). Click any cell in the Table to edit in place; drag cards on the Kanban to update the underlying field
- **Saved views** — name a filter / sort / group / column-set / view-mode bundle as a tab; switch between Notion-shaped views with one click. Stored in `.eduport/views.yaml` (synced)
- **Filter, sort, group-by, collapsible sections** — every list view supports per-property filters and sort, plus collapsible group-by-select sections. Sidebar shows per-property chip aggregations with counts
- **Wikilinks as the relationship graph** — `[[eth-zurich-K9p3]]` references resolve by id-suffix, so renames in Obsidian don't break links
- **⌘K command palette** — quick lookup + full-text search via SQLite FTS5 (covers custom text/url property values too)
- **First-run onboarding** — picks a data folder, creates `attachments/` and `notes/` subfolders, optional sample seeds
- **Soft delete** — items move to `<data folder>/.eduport-trash/`, restorable from the in-app Trash view
- **Source of truth = your files** — SQLite lives outside the data folder (in the OS cache dir) and rebuilds itself if missing or stale

## Architecture

```
┌─────────────────────────────────────────┐
│ Tauri shell (Rust) — eduport-tauri       │
│  • Hosts the WebView                     │
│  • #[tauri::command] handlers wrap        │
│    eduport-core for the SvelteKit UI     │
│  • Forwards watcher events as Tauri      │
│    events (eduport:vault-event /         │
│    eduport:parse-error)                  │
└─────────────┬───────────────────────────┘
              │ Tauri command channel (in-process)
              ▼
┌─────────────────────────────────────────┐
│ eduport-core (Rust library)              │
│  • Typed entities + custom-property       │
│    schema validator                      │
│  • Atomic file writes via vaultdb-core   │
│  • SQLite + FTS5 search/filter index      │
│  • notify-based file watcher             │
│  • EML parser                            │
└─────────────┬───────────────────────────┘
              │ depends on
              ▼
┌─────────────────────────────────────────┐
│ vaultdb-core (Rust library)              │
│  • Treats a markdown vault as a queryable │
│    database                              │
│  • Frontmatter parser, link graph,        │
│    transactional rename, vault lock       │
└─────────────────────────────────────────┘
```

> The Python (FastAPI) sidecar that this stack used to ship with was
> retired in rewrite phase 11; everything now lives in process via
> `eduport-core` and `eduport-tauri`. The original design spec is at
> [`docs/superpowers/specs/2026-05-09-vaultdb-rewrite-design.md`](docs/superpowers/specs/2026-05-09-vaultdb-rewrite-design.md).

## Tech stack

| Layer | Choice |
|---|---|
| Native shell | Tauri 2 (Rust) |
| Domain layer | `eduport-core` (Rust) over `vaultdb-core` |
| Frontend | SvelteKit + Svelte 5, Tailwind CSS v4, CodeMirror 6 |
| Markdown render (UI) | `marked` + custom wikilink/checkbox extraction |
| File watcher | `notify` + `notify-debouncer-full` (Rust) |
| Storage / search | `rusqlite` with bundled SQLite + FTS5 |
| Desktop bundling | Tauri (no external sidecar binary) |
| Tooling | `cargo` (Rust), `npm` (frontend) |

## Project structure

```
Cargo.toml                     workspace manifest
crates/
  eduport-core/                Rust library — typed schema, FTS5, watcher, EML, settings
  eduport-tauri/               Rust shell (Tauri 2) — Tauri commands + native bridges
docs/                          design spec, packaging notes, implementation plans
frontend/                      SvelteKit app — UI + Tauri-invoke API client
scripts/                       build helpers (Tauri prereqs)
```

## Prerequisites

The project targets macOS, Windows, and Linux. All three need:

- **Rust** 1.77.2+ (`rustup install stable`)
- **Node.js** 20+ and npm

Plus the OS-specific Tauri prerequisites (full reference: <https://v2.tauri.app/start/prerequisites/>):

- **macOS:** Xcode Command Line Tools — `xcode-select --install`
- **Windows:** [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (with the "Desktop development with C++" workload). WebView2 is preinstalled on Windows 10/11.
- **Linux (Debian/Ubuntu):** `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `librsvg2-dev`, `build-essential`

## Getting started

### Run the frontend in a browser (UI only, no Tauri bridges)

```bash
cd frontend
npm install
npm run dev          # http://localhost:5173
```

In browser-only mode, dialogs and API commands that require Tauri fall back to dev placeholders or throw — the dev server is for component work, not API exercise.

### Build + install the full desktop app

The packaging script builds the SvelteKit frontend then runs `tauri build`:

```bash
python3 scripts/build_desktop.py
```

On Linux this produces `.deb` and `.rpm` under `target/release/bundle/`. Install with `sudo apt install ./target/release/bundle/deb/eduport_*.deb` (or your distro's equivalent). See [docs/packaging.md](docs/packaging.md) for advanced bundle targets and CI notes.

## Tests + checks

```bash
# eduport-core unit tests
cargo test -p eduport-core

# whole-workspace clippy
cargo clippy --workspace --all-targets -- -D warnings

# Frontend — type-check + Svelte diagnostics
cd frontend
npm run check
npx vitest --run
```

## Documentation

- **[Design spec](docs/superpowers/specs/2026-05-06-eduport-design.md)** — entity model, storage layout, UI shape, sync semantics, parse-error handling — the canonical "why" and "what"
- **[Packaging](docs/packaging.md)** — local builds, GitHub Actions workflow, signing notes
- **Implementation plans** — granular per-layer breakdowns under [`docs/superpowers/plans/`](docs/superpowers/plans/)

## License

MIT (per the workspace `Cargo.toml`). A standalone `LICENSE` file is on the to-do list.
