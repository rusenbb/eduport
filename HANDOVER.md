# Handover for the next Claude session

Hi. I'm the Claude that just finished the v0.1.0 → v0.1.1 rewrite
of this repo. You're picking up after that. A few things to know
before you change anything.

## The current state of the world

- **`main` is at `f66fc02`** and is pushed to
  <https://github.com/rusenbb/eduport>. CI is green.
- The **Python sidecar is gone**. Everything that used to run as
  a separate FastAPI process now runs in-process via Tauri
  commands. Don't bring the sidecar back; that decision is
  load-bearing (134 MiB of bundle weight saved, no version-drift
  risk with system SQLite, no HTTP loopback to debug).
- **`eduport-core`** is the Rust library that holds every piece
  of domain logic (typed entities, schema validator, FTS5
  search, notify watcher, EML import). It sits on
  **`vaultdb-core`** (a separate co-developed library at
  `/home/rusen/Desktop/codebase-shared/researches/vaultdb/`).
  Path-dep with a `version = "1.0.0"` companion so local dev
  uses the path and `cargo publish` uses the version.
- **`eduport-tauri`** is the Tauri shell. It owns 35
  `#[tauri::command]` handlers that wrap `eduport-core` for the
  SvelteKit frontend, plus the boot path that reads
  `settings.toml`, opens the vault, runs reconcile, and starts
  the watcher.
- **`frontend/`** talks to the Tauri shell via `coreInvoke()`
  (helper in `src/lib/api/client.ts`). No HTTP. No legacy
  `apiFetch`.
- Test counts as of `f66fc02`: 123 eduport-core, 174
  vaultdb-core, 10 frontend (vitest). Clippy clean under
  `-D warnings`. `svelte-check` 449 files / 0 errors / 0 warnings.

## The one thing pending live verification

The most recent commit (the flat-root-layout fix) hasn't been
**installed and exercised against the real vault yet**. Earlier
in the previous session the user discovered that a real eduport
vault stores every entity flat at the data folder root,
discriminated by the file's `eduport-type/<value>` tag — not in
per-type subfolders. The previous port had a `FolderMap` that
assumed subfolders. The fix landed but the user hadn't yet
installed the new `.deb` over the v0.1.0 installed package.
Install path:

```bash
sudo apt remove eduport -y && \
sudo apt install /home/rusen/Desktop/codebase-shared/rusen/eduport/target/release/bundle/deb/Eduport_0.1.1_amd64.deb -y
```

The user is on Linux Mint (or similar — uses .deb). The launcher
icon points at `/usr/bin/eduport` from the installed package; you
can also run `./target/release/eduport` directly without
installing if you want a quick test.

## Conventions that aren't obvious from the code

1. **Tag-discriminated, root-flat layout.** Every entity is a
   `.md` file at the vault root. Type comes from the
   `eduport-type/<value>` tag inside its YAML frontmatter. Files
   in subfolders (`notes/`, `attachments/`) are user-managed
   Obsidian content and are **not** eduport entities — never
   add code that walks subfolders looking for entities, you'll
   pick up the user's scratch notes.

2. **vaultdb-core is the substrate; don't push eduport-specific
   concerns into it.** The library is meant to be reusable
   (research-notes apps, knowledge bases). If you're adding
   something that only eduport cares about, it goes in
   `eduport-core`.

3. **Watcher events are type-agnostic.** `VaultEvent::EntityChanged`
   and `EntityDeleted` carry `path` + `file_id` only. The
   consumer parses the frontmatter to discover type. If you find
   yourself wanting to put `kind: EntityType` back on these
   events, you're probably about to make the watcher too smart.

4. **Mutators call `Watcher::note_self_write(path)` before they
   touch disk**, so the watcher doesn't bounce our own writes
   through the parser. Every place that writes an entity file —
   `entity::create/update/delete` commands, `purge_orphans`,
   `core_checkbox_toggle` — does this. If you add a new write
   path, do the same.

5. **The frontend's error type is `CoreCommandError`** with a
   stable `code` (`invalid` / `not_found` / `conflict` /
   `internal` / `not_initialised`). Branch on the code, not the
   message. The Rust side serialises via `CommandError` in
   `eduport-tauri/src/commands/mod.rs`.

## Tag namespaces in this repo

- `desktop-v*` — releases the Tauri installers via
  `.github/workflows/release.yml`. The user pushes one when
  they want a new desktop release.
- `eduport-core-v*` — publishes the `eduport-core` crate to
  crates.io via `.github/workflows/publish-eduport-core.yml`.
  Gated behind the `production` Environment for one-click
  approval. The secret `CARGO_REGISTRY_TOKEN` was **not yet
  added** as of session end — needs to be added at
  <https://github.com/rusenbb/eduport/settings/secrets/actions>
  before the first publish.

The matching tags in the vaultdb repo are `v*` — that repo has
`v1.0.0` tagged locally but **not pushed**. Once vaultdb-core
v1.0.0 is on crates.io, the side-by-side checkout step in
eduport's CI becomes redundant; the path-dep falls back to the
version automatically. There's a TODO comment in
`.github/workflows/ci.yml` about removing it then.

## What the user wants you to work on

"Make eduport better." That's open-ended. Some likely
directions surfaced in the prior session:

- **The frontend Status page still has stale "sidecar" wording**
  in its subtitle. Trivial copy fix — grep
  `frontend/src/routes/status` for "sidecar".
- **Verify the rest of the surface against the real vault**
  (entity create / edit / delete, schema editor, saved views,
  search, EML import). Only the universities-list path has been
  exercised end-to-end on real data so far.
- **The user has 3 .md files in `notes/`** that are pre-existing
  plain markdown without frontmatter. They show up as parse
  errors. Decide whether the Status page should let users
  dismiss them, or whether reconcile should ignore unparseable
  files in subfolders that aren't entity candidates anyway.
- **vaultdb-core v1.0.0 hasn't been published yet.** Either
  push the tag (triggers the gated publish workflow) or leave
  it. The user previously chose "publish only the substrate"
  (vaultdb-core + vaultdb-mcp + vaultdb CLI), explicitly
  skipping PyPI + npm. Don't surface those again unless asked.
- **`vaultdb-pyo3`** and **`vaultdb-wasm`** crates exist in the
  vaultdb workspace as research artifacts. They build in CI
  but aren't published. Don't change that unless the user asks.

## Things the user has told me explicitly

- Don't include "Co-Authored-By" / AI credit in commit messages
  or PR bodies. Their global CLAUDE.md is strict about this.
- Always ask before publishing to public registries, force-pushing,
  pushing tags, or any other irreversible externally-visible
  action.
- Prefer concrete file paths + line numbers when reporting
  findings; their CLAUDE.md asks for moderately-detailed,
  reasoning-shown answers (not terse ones).
- When the user asks "is X better now?" they want concrete
  metrics (test counts, binary size, etc.) plus an actual demo
  of the running app — not just an "all green" report.

## Gotchas you might step on

- **CI rust toolchain drifts ahead of the local toolchain.**
  Lints surfaced in the previous session that didn't reproduce
  locally (rust 1.92 local vs 1.95 on the runner). Re-run CI
  after pushing; fix lints as they surface rather than pinning.
- **eduport-tauri is on Rust edition 2021**; eduport-core is
  edition 2024. `let-chains` work in core but not in tauri —
  use nested `if let` blocks on the tauri side.
- **`cargo package` for eduport-core strips the path-dep and
  resolves through the registry**, which fails until
  vaultdb-core v1.0.0 actually lands on crates.io. CI dry-run
  for eduport-core is intentionally skipped right now.
- **Tauri's `beforeBuildCommand` cwd is platform-dependent.**
  Earlier macOS CI runs failed where Linux passed. The current
  setup runs `npm --prefix frontend run build` from
  `build_desktop.py` directly instead of through
  `tauri.conf.json:beforeBuildCommand`. Don't put it back.
- **Tauri bundles land at `target/release/bundle/`** (workspace
  root), not at `crates/eduport-tauri/target/release/bundle/`.
  CI paths reference the workspace target.

## Co-developed repos

- `/home/rusen/Desktop/codebase-shared/rusen/eduport/` — this
  repo, `main` at `f66fc02`.
- `/home/rusen/Desktop/codebase-shared/researches/vaultdb/` —
  the substrate, `main` at `dee1960`, has a local `v1.0.0` tag
  that hasn't been pushed.

If you change `vaultdb-core`'s public API, expect to also touch
`eduport-core` (which is the only consumer in-tree). The
co-development pattern is path-deps for now, version specs for
the eventual publish.

Good luck. The codebase is in pretty good shape and the
maintenance load should be light unless you go after a big new
feature.

— Previous Claude
