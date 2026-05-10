# Releasing eduport

This repo ships **two distinct artifacts** with their own version
cadences:

1. **The desktop app** — Tauri shell + frontend bundle. Released
   as `.deb` / `.rpm` / `.AppImage` / `.dmg` / `.msi` / `.exe`
   via GitHub Releases. Tag namespace: `desktop-v*`.
2. **`eduport-core` (the Rust library)** — published to crates.io
   for downstream consumers (a future CLI, library users). Tag
   namespace: `eduport-core-v*`.

The two tag namespaces are intentionally distinct so a desktop
release doesn't accidentally publish the library, and vice
versa. `git tag --list` groups them visibly.

## Pre-flight (always)

```bash
cargo test -p eduport-core
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
npm --prefix frontend run check
npm --prefix frontend exec vitest run
```

Or push to a branch and let `.github/workflows/ci.yml` run all of
the above — it's fast (~3 min) and pre-clears the heavier Tauri
build.

## Releasing the desktop app

```bash
# 1. Bump the version in crates/eduport-tauri/tauri.conf.json
#    AND in the workspace Cargo.toml (`[workspace.package].version`).
#    Commit + push.

# 2. Tag with the `desktop-v*` prefix.
git tag -a desktop-v0.1.2 -m "eduport desktop v0.1.2"
git push origin desktop-v0.1.2
```

`.github/workflows/release.yml` picks up the tag, builds installers
on Linux + macOS + Windows, and opens a **draft** GitHub Release
with the assets attached. Sanity-check the assets, then click
**Publish release** in the GitHub UI.

`.github/workflows/desktop-build.yml` runs the same Tauri bundle
build on every push to `main` (no upload), so the release-time
build rarely surprises.

## Releasing eduport-core to crates.io

Pre-requisite: `vaultdb-core` v1.0.0 (the dep eduport-core resolves
to via `path + version`) must already be on crates.io. See the
vaultdb repo's `RELEASE.md` for that flow.

```bash
# 1. Bump the workspace version in Cargo.toml.
#    Commit + push.

# 2. Tag with the `eduport-core-v*` prefix.
git tag -a eduport-core-v0.1.1 -m "eduport-core v0.1.1"
git push origin eduport-core-v0.1.1
```

`.github/workflows/publish-eduport-core.yml` picks up the tag,
pauses on the `production` GitHub Environment for one-click
approval, then runs `cargo publish -p eduport-core`. A draft
GitHub Release for the tag follows.

## One-time GitHub setup

Done once per repo, not per release:

- **Secret**: `CARGO_REGISTRY_TOKEN` — from
  <https://crates.io/me/account/tokens> with publish scope. Add
  under Settings → Secrets and variables → Actions.
- **Environment**: `production` — Settings → Environments → New
  environment → "production" → Required reviewers → add yourself.
  This is what gives you the one-click gate before any
  `cargo publish` runs.

## Yanking a bad release

```bash
# crates.io
cargo yank --version 0.1.1 eduport-core

# GitHub Release: edit and unpublish from the UI; keep the tag
# so version history reads cleanly.
```

The version slot is gone forever — the next release must
increment.
