# Eduport Tauri Shell — Implementation Plan (Plan 3)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans.

**Goal:** Wrap the Plan 1 sidecar + Plan 2 frontend in a Tauri desktop window. End-state: `npm run tauri dev` produces a real desktop window that auto-spawns `eduport-sidecar`, polls `/health`, loads the SvelteKit build, and exposes native dialogs (open / save / reveal-in-file-manager / `.eml` drag-and-drop). Production build via `npm run tauri build` produces per-OS installers.

**Architecture:** Tauri 2 (Rust) wraps the SvelteKit static build. Tauri spawns the Python sidecar as a child process and supervises its lifecycle. The frontend reaches the sidecar via HTTP loopback on a port the Tauri shell selects at startup and injects as `VITE_SIDECAR_URL` (via window.__EDUPORT_API_URL__ or a build-time env). Tauri commands handle the bits the WebView can't: native dialogs, OS-trash integration (already partly done in sidecar via `send2trash`, but the "reveal in file manager" path is shell-only), `.eml` drop hookup, Obsidian URI launch.

**Tech Stack:** Tauri 2, Rust 2021 edition, `tauri-plugin-dialog`, `tauri-plugin-shell`, `tauri-plugin-fs`, `tauri-plugin-process` (for sidecar lifecycle).

---

## File Structure

```
src-tauri/                  # Tauri shell at repo root
├── Cargo.toml
├── tauri.conf.json
├── build.rs
├── src/
│   ├── main.rs             # entry, plugin wiring
│   ├── sidecar.rs          # spawn + supervise eduport-sidecar
│   ├── reveal.rs           # OS-specific "reveal in file manager"
│   └── obsidian.rs         # build & launch obsidian:// URI
└── icons/
    └── ...                 # Tauri's standard icon set
```

**Important:** the sidecar binary needs to be embedded. We use `tauri.conf.json`'s `bundle.externalBin` or `tauri.conf.json`'s `bundle.resources` referencing a PyInstaller-built executable. v1 development uses `eduport-sidecar` from the system PATH; production packaging is a follow-up.

---

## Task 0: Tauri scaffold

**Files:** `src-tauri/*` (created by `npm create tauri-app@latest` invoked inside an existing project).

- [ ] From repo root: `npm install -D @tauri-apps/cli`. Then `npx @tauri-apps/cli init` and answer prompts: app name `Eduport`, window title `Eduport`, frontend dist dir `../frontend/build`, dev URL `http://localhost:5173`, frontend dev command `npm --prefix frontend run dev`, frontend build command `npm --prefix frontend run build`.
- [ ] Verify: `npm run tauri dev` boots a window showing the SvelteKit dev page.
- [ ] Add `src-tauri/target/` to repo `.gitignore` (probably already covered).
- [ ] Commit: `chore(shell): scaffold Tauri 2 around frontend`.

---

## Task 1: Sidecar lifecycle

**Files:** `src-tauri/src/sidecar.rs`, `src-tauri/src/main.rs`, `src-tauri/Cargo.toml`.

- [ ] Add deps: `tokio = { version = "1", features = ["full"] }`, `reqwest = { version = "0.12", features = ["json"] }`, `tauri-plugin-shell`.
- [ ] `sidecar.rs`: function `spawn_sidecar() -> Result<(Child, u16)>` — pick a free localhost port (`tokio::net::TcpListener::bind("127.0.0.1:0").local_addr()`), spawn `eduport-sidecar --port <port>` via `std::process::Command`, return the child + port.
- [ ] `health_check_loop(port, max_seconds=5)` — poll `GET /health` on 100ms ticks until 200, return Ok(()) or timeout-error.
- [ ] In `main.rs`: in `setup` callback, call `spawn_sidecar`, await `health_check_loop`, then store the port in app state. On error, show a Tauri error dialog with the sidecar log path (resolved via `tauri::api::path::log_dir`).
- [ ] On app exit: kill the sidecar (Tauri's `RunEvent::ExitRequested` handler).
- [ ] Inject the port into the window: in `main.rs`, after sidecar is up, call `window.eval(&format!("window.__EDUPORT_API_URL__ = 'http://127.0.0.1:{}'", port))`.
- [ ] Frontend side: edit `frontend/src/lib/api/client.ts` to read `window.__EDUPORT_API_URL__` (with `import.meta.env.VITE_SIDECAR_URL` as the dev fallback).
- [ ] Smoke: `npm run tauri dev` boots the shell, sidecar starts, frontend appears with data flowing.
- [ ] Commit: `feat(shell): sidecar spawn + health check + URL injection`.

---

## Task 2: Native dialogs (open / save / folder picker)

**Files:** edit `src-tauri/src/main.rs` to register `tauri-plugin-dialog`; edit relevant frontend components.

- [ ] Add `tauri-plugin-dialog` to `Cargo.toml`, register in `main.rs`.
- [ ] Frontend: in the first-run flow (Plan 2 Task 18), replace the typed-path input with `import { open } from '@tauri-apps/plugin-dialog'; const folder = await open({ directory: true })`.
- [ ] In Document detail panel: "Save copy as…" calls `import { save } from '@tauri-apps/plugin-dialog'` and copies the binary via Tauri fs plugin.
- [ ] Commit: `feat(shell): native folder picker + save dialog wired into frontend`.

---

## Task 3: Reveal in file manager

**Files:** `src-tauri/src/reveal.rs`, register a Tauri command in `main.rs`.

- [ ] `reveal_in_finder(path: String)` — `#[tauri::command]` that runs:
  - macOS: `open -R <path>`
  - Windows: `explorer.exe /select,<path>`
  - Linux: `xdg-open <parent of path>` (closest portable approximation; selection-aware variants vary by file manager)
- [ ] Frontend: Document detail panel's "Reveal in file manager" button calls `invoke('reveal_in_finder', { path })`.
- [ ] Commit: `feat(shell): reveal-in-file-manager native bridge`.

---

## Task 4: Open-in-Obsidian URI

**Files:** `src-tauri/src/obsidian.rs`, command registration.

- [ ] `open_in_obsidian(vault: String, file: String)` — uses `tauri-plugin-shell`'s `open` to launch `obsidian://open?vault={vault}&file={file}`. Vault name is configured in settings (Plan 1 already has the field — actually it doesn't; add `obsidian_vault: Optional[str]` to `Settings` and a corresponding settings UI input).
- [ ] If `obsidian_vault` is not configured, the button is disabled with a tooltip explaining how to set it.
- [ ] Commit: `feat(shell): obsidian:// URI launcher`.

---

## Task 5: `.eml` drag-and-drop hookup

**Files:** edit `frontend/src/lib/components/CommandPalette.svelte` (or wherever the drop target is) + register the Tauri drag-and-drop event.

- [ ] Tauri 2 emits `tauri://file-drop` events. In `+layout.svelte`, listen via `import { listen } from '@tauri-apps/api/event'` and on a `.eml` drop, read the file via Tauri fs plugin and POST its bytes to `/eml/parse`.
- [ ] Open the `EntityForm` for an Email entity pre-filled with the parsed payload.
- [ ] Commit: `feat(shell): .eml drag-and-drop opens prefilled email form`.

---

## Task 6: Settings persistence at the shell level

**Files:** edit `sidecar/src/eduport/cli.py` and add startup behaviour.

- [ ] On Tauri shell startup, before spawning the sidecar, check whether `<config dir>/settings.toml` exists (resolved via `tauri::api::path::config_dir`). If not, prompt the user (folder picker + email input), then write the TOML file using the same format as the sidecar's `save_settings`. Then spawn the sidecar.
- [ ] **Or** — simpler — the sidecar refuses to start without settings (Plan 1 already does this with exit code 2). The shell intercepts that exit code, prompts the user, writes the file, retries the spawn.
- [ ] Pick whichever feels less coupled. The simpler path is to let the sidecar own the settings file format; the shell only knows it needs to write the file when missing.
- [ ] Commit: `feat(shell): first-run prompts for data folder + email, writes settings.toml`.

---

## Task 7: Production build verification

- [ ] Run `npm run tauri build`. Verify a per-OS bundle is produced.
- [ ] **Sidecar packaging note (deferred):** the production bundle assumes `eduport-sidecar` is on PATH. For a true single-installer experience, Plan 4 (or a follow-up to Plan 3) does PyInstaller bundling + `bundle.externalBin` wiring. v1 ships with the requirement that the user installs the sidecar separately (`uv tool install ./sidecar` or similar).
- [ ] Smoke: install the bundled app on the dev machine, launch, verify it reaches the sidecar and renders the dashboard.
- [ ] Commit: `chore(shell): production build verified on $(uname)`.

---

## Self-review

**Spec coverage** vs design spec §6 + §7:
- §6 architecture diagram — Task 0 + Task 1.
- §6.1 bootstrap sequence — Task 1 (port selection, health polling, retry, error UI).
- §7.4 document actions (Open / Reveal / Save copy) — Tasks 2 + 3.
- §7.7 first-run — Task 6.
- §7.2 "Open in Obsidian" — Task 4.
- §8.5 `.eml` drag-and-drop — Task 5.

**What this plan does NOT cover (deferred to a Plan 4 / packaging plan):**
- PyInstaller / PyOxidizer bundling of the sidecar into the app binary.
- Code signing for macOS / Windows.
- Auto-update via Tauri's updater plugin.
- Cross-compilation (each OS builds locally for v1).

**Pacing.** This plan is the smallest of the three because the heavy logic lives in the sidecar (Plan 1) and the frontend (Plan 2). The shell's job is glue, native bridges, and lifecycle.
