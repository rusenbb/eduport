# Phase 1 — vaultdb Workspace Split — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Split the existing vaultdb binary-only crate into a Cargo workspace containing `vaultdb-core` (library — engine modules) and `vaultdb-cli` (binary — CLI wrappers), with no behaviour change. Today's CLI test suite passes unchanged.

**Architecture:** Pure structural refactor. The 8 engine modules (`error`, `record`, `frontmatter`, `links`, `filter`, `schema`, `vault`, `writer`) move to `crates/vaultdb-core/src/`. The CLI-specific modules (`cli`, `output`, `main`, `commands/*`) stay in `crates/vaultdb-cli/src/`. `vaultdb-cli` depends on `vaultdb-core` via `path = "../vaultdb-core"`.

**Tech Stack:** Rust 2024 edition, `cargo` workspaces, `clap` (CLI), `serde_yaml`, `serde_json`, `walkdir`, `regex`, `comfy-table`, `colored`, `anyhow`, `thiserror`, `csv`. Dev-deps: `tempfile`, `assert_cmd`, `predicates`.

**Working directory for all tasks:** `/home/rusen/Desktop/codebase-shared/researches/vaultdb`. This is a different repo from the eduport one this plan lives in. **All `cargo` commands and `git` commands in this plan are run from there**, not from eduport's working tree.

**Non-goals for phase 1:**
- No public API changes (the `FieldValue` → `Value` rename, the `Expr` AST split, the `LoadResult` parse-diagnostics struct — all deferred to phase 2).
- No new behaviour. The CLI runs every command exactly as it does today, with the same flags, output, and exit codes.
- No new lib crate consumers. `vaultdb-mcp` arrives in phase 3; `eduport-core` arrives in phase 4. This phase only ships the workspace shape.

---

## File structure after this phase

```
researches/vaultdb/
├── Cargo.toml                     workspace manifest (new shape)
├── Cargo.lock                     regenerated
├── README.md                      unchanged
├── LICENSE                        unchanged
├── target/                        gitignored
├── skills/                        unchanged (Claude Code skill)
└── crates/
    ├── vaultdb-core/
    │   ├── Cargo.toml             package manifest (lib only)
    │   └── src/
    │       ├── lib.rs             pub mod declarations for engine modules
    │       ├── error.rs           moved from src/error.rs
    │       ├── record.rs          moved from src/record.rs
    │       ├── frontmatter.rs     moved from src/frontmatter.rs
    │       ├── links.rs           moved from src/links.rs
    │       ├── filter.rs          moved from src/filter.rs
    │       ├── schema.rs          moved from src/schema.rs
    │       ├── vault.rs           moved from src/vault.rs
    │       └── writer.rs          moved from src/writer.rs
    └── vaultdb-cli/
        ├── Cargo.toml             package manifest (binary)
        └── src/
            ├── main.rs            moved from src/main.rs (with reduced mod list)
            ├── cli.rs             moved from src/cli.rs
            ├── output.rs          moved from src/output.rs
            └── commands/
                ├── mod.rs         moved from src/commands/mod.rs
                ├── create.rs      moved unchanged
                ├── delete.rs      moved unchanged
                ├── links.rs       moved unchanged
                ├── move_cmd.rs    moved unchanged
                ├── query.rs       moved unchanged
                ├── rename.rs      moved unchanged
                ├── schema_cmd.rs  moved unchanged
                ├── traverse.rs    moved unchanged
                ├── unresolved.rs  moved unchanged
                └── update.rs      moved unchanged
```

The old `src/` directory disappears completely. All Cargo.toml metadata (description, version, etc.) ends up in the per-crate `Cargo.toml`s; the workspace `Cargo.toml` only declares members.

---

## Task 1: Convert vaultdb to a Cargo workspace, move all current code into `crates/vaultdb-cli/`

**Goal:** Move the existing binary code wholesale into `crates/vaultdb-cli/` without changing any of its internals. After this task, the binary still has every module, every test, every behaviour — just in a new location. No `vaultdb-core` yet.

**Files:**
- Create: `crates/vaultdb-cli/Cargo.toml`
- Move: `src/` → `crates/vaultdb-cli/src/`
- Modify: `Cargo.toml` (becomes a workspace manifest)

- [ ] **Step 1: Create `crates/vaultdb-cli/`**

```bash
cd /home/rusen/Desktop/codebase-shared/researches/vaultdb
mkdir -p crates/vaultdb-cli
git mv src crates/vaultdb-cli/src
```

- [ ] **Step 2: Write `crates/vaultdb-cli/Cargo.toml` with the existing package metadata**

Replace the file at `crates/vaultdb-cli/Cargo.toml` with:

```toml
[package]
name = "vaultdb-cli"
version = "0.1.0"
edition = "2024"
description = "CLI for vaultdb — database-like operations on Obsidian markdown files"
license = "MIT"
repository = "https://github.com/rusenbb/vaultdb"
readme = "../../README.md"
keywords = ["obsidian", "markdown", "database", "cli", "knowledge-graph"]
categories = ["command-line-utilities", "database"]

[[bin]]
name = "vaultdb"
path = "src/main.rs"

[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
serde_json = "1"
regex = "1"
walkdir = "2"
comfy-table = "7"
colored = "2"
anyhow = "1"
thiserror = "2"
csv = "1"

[dev-dependencies]
tempfile = "3"
assert_cmd = "2"
predicates = "3"
```

The `[[bin]]` `name = "vaultdb"` ensures `cargo install` produces a binary called `vaultdb`, not `vaultdb-cli`.

- [ ] **Step 3: Replace the root `Cargo.toml` with a workspace manifest**

Overwrite `Cargo.toml` (in the vaultdb repo root) with:

```toml
[workspace]
resolver = "2"
members = [
    "crates/vaultdb-cli",
]

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "MIT"
repository = "https://github.com/rusenbb/vaultdb"

[profile.release]
lto = true
codegen-units = 1
strip = true
```

`resolver = "2"` is required for edition 2024 in a workspace context. The `[profile.release]` block is workspace-wide (applies to every member's release builds).

- [ ] **Step 4: Run `cargo build --workspace`**

```bash
cargo build --workspace
```

Expected: builds successfully. The binary is at `target/debug/vaultdb`. If you see "could not find Cargo.toml" errors mentioning `src/`, double-check that the `src/` directory is fully gone from the repo root and lives under `crates/vaultdb-cli/`.

- [ ] **Step 5: Run `cargo test --workspace`**

```bash
cargo test --workspace
```

Expected: all unit tests in the modules pass. The output should look something like:

```
running N tests
test record::tests::... ok
test frontmatter::tests::... ok
test vault::tests::... ok
...
test result: ok. N passed; 0 failed
```

If a test fails, it is **not** because of the move — the move did not change any code. Investigate as a real failure (likely a stale `target/` directory that needs `cargo clean`, or a path-sensitive test).

- [ ] **Step 6: Smoke-test the binary**

```bash
./target/debug/vaultdb --help
```

Expected: shows the existing CLI help text (the same one as before the move).

- [ ] **Step 7: Commit**

```bash
git add -A
git status
git commit -m "refactor(workspace): move all source into crates/vaultdb-cli/

No code or behaviour change. The workspace currently has one member;
crates/vaultdb-core/ arrives in the next task. This commit only
relocates files and converts the root Cargo.toml to a workspace manifest."
```

`git status` before the commit should show `src/ -> crates/vaultdb-cli/src/` rename detection (Git tracks moves) and the two Cargo.toml changes. If `git status` shows large blobs of additions/deletions instead of renames, that's fine — Git's rename detection is heuristic and the result is the same either way.

---

## Task 2: Add an empty `vaultdb-core` library crate

**Goal:** Scaffold `crates/vaultdb-core/` as a library crate with an empty `lib.rs`. Add it to the workspace. No engine modules moved yet.

**Files:**
- Create: `crates/vaultdb-core/Cargo.toml`
- Create: `crates/vaultdb-core/src/lib.rs`
- Modify: `Cargo.toml` (workspace manifest — add new member)

- [ ] **Step 1: Create the directory and files**

```bash
mkdir -p crates/vaultdb-core/src
```

- [ ] **Step 2: Write `crates/vaultdb-core/Cargo.toml`**

Create file `crates/vaultdb-core/Cargo.toml`:

```toml
[package]
name = "vaultdb-core"
version = "0.1.0"
edition = "2024"
description = "Library engine for vaultdb — markdown-as-database for Obsidian-style vaults"
license = "MIT"
repository = "https://github.com/rusenbb/vaultdb"
readme = "../../README.md"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
serde_json = "1"
regex = "1"
walkdir = "2"
thiserror = "2"

[dev-dependencies]
tempfile = "3"
```

The dependency list is the engine subset of `vaultdb-cli`'s deps. Notably absent: `clap` (CLI-only), `comfy-table` (output formatter), `colored` (output formatter), `csv` (output formatter), `anyhow` (the library exposes `VaultdbError` only — `anyhow` belongs in the binary), `assert_cmd` and `predicates` (CLI integration testing).

- [ ] **Step 3: Write `crates/vaultdb-core/src/lib.rs` as an empty stub**

Create file `crates/vaultdb-core/src/lib.rs`:

```rust
//! vaultdb-core — library engine for vaultdb.
//!
//! Engine modules will be moved here in the next task.
```

- [ ] **Step 4: Add `vaultdb-core` to the workspace `members` list**

Edit the root `Cargo.toml`. The `[workspace]` block should now read:

```toml
[workspace]
resolver = "2"
members = [
    "crates/vaultdb-core",
    "crates/vaultdb-cli",
]
```

- [ ] **Step 5: Verify the workspace builds**

```bash
cargo build --workspace
```

Expected: both crates build. `vaultdb-core` produces nothing observable (an empty library). `vaultdb-cli` produces the binary as before.

- [ ] **Step 6: Verify tests still pass**

```bash
cargo test --workspace
```

Expected: same N tests pass as in Task 1, Step 5.

- [ ] **Step 7: Commit**

```bash
git add -A
git status
git commit -m "feat(workspace): scaffold empty vaultdb-core library crate

Adds crates/vaultdb-core/ as a workspace member with a stub lib.rs.
Engine modules will move here in the next commit. No behaviour change."
```

---

## Task 3: Move foundation modules (`error`, `record`) to `vaultdb-core`

**Goal:** Move the two no-internal-deps engine modules into `vaultdb-core`. Update `vaultdb-cli` to import them from the new crate.

These two are the bottom of the dependency tree (`error.rs` has zero internal `use crate::` imports; `record.rs` has zero internal `use crate::` imports). Moving them first means every subsequent move has somewhere to depend on.

**Files:**
- Move: `crates/vaultdb-cli/src/error.rs` → `crates/vaultdb-core/src/error.rs`
- Move: `crates/vaultdb-cli/src/record.rs` → `crates/vaultdb-core/src/record.rs`
- Modify: `crates/vaultdb-core/src/lib.rs`
- Modify: `crates/vaultdb-cli/Cargo.toml`
- Modify: `crates/vaultdb-cli/src/main.rs`
- Modify: every file in `crates/vaultdb-cli/src/` that uses `crate::error` or `crate::record`

- [ ] **Step 1: Move the files**

```bash
cd /home/rusen/Desktop/codebase-shared/researches/vaultdb
git mv crates/vaultdb-cli/src/error.rs crates/vaultdb-core/src/error.rs
git mv crates/vaultdb-cli/src/record.rs crates/vaultdb-core/src/record.rs
```

- [ ] **Step 2: Expose them from `vaultdb-core`'s `lib.rs`**

Replace `crates/vaultdb-core/src/lib.rs` with:

```rust
//! vaultdb-core — library engine for vaultdb.

pub mod error;
pub mod record;
```

- [ ] **Step 3: Add `vaultdb-core` as a dependency of `vaultdb-cli`**

Edit `crates/vaultdb-cli/Cargo.toml`. Under `[dependencies]`, add:

```toml
vaultdb-core = { path = "../vaultdb-core" }
```

The `[dependencies]` block should now look like:

```toml
[dependencies]
vaultdb-core = { path = "../vaultdb-core" }
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
serde_json = "1"
regex = "1"
walkdir = "2"
comfy-table = "7"
colored = "2"
anyhow = "1"
thiserror = "2"
csv = "1"
```

- [ ] **Step 4: Remove `mod error;` and `mod record;` from `vaultdb-cli`'s `main.rs`**

Edit `crates/vaultdb-cli/src/main.rs`. The top of the file currently reads:

```rust
#![allow(dead_code)]

mod cli;
mod commands;
mod error;
mod filter;
mod frontmatter;
mod links;
mod output;
mod record;
mod schema;
mod vault;
mod writer;
```

Change it to remove the two moved modules:

```rust
#![allow(dead_code)]

mod cli;
mod commands;
mod filter;
mod frontmatter;
mod links;
mod output;
mod schema;
mod vault;
mod writer;
```

(The other `mod` lines are still required because those modules haven't been moved yet — they will be in tasks 4 and 5.)

- [ ] **Step 5: Sweep-replace `crate::error` and `crate::record` in `vaultdb-cli` source files**

Run a search to find every consumer:

```bash
cd /home/rusen/Desktop/codebase-shared/researches/vaultdb
grep -rEn "crate::(error|record)" crates/vaultdb-cli/src/
```

For each match, change `crate::error` → `vaultdb_core::error` and `crate::record` → `vaultdb_core::record`. The crate name uses an underscore in import paths (`vaultdb_core`), not a hyphen.

The files to update (based on grep output you'll see) include:
- `crates/vaultdb-cli/src/filter.rs` (uses `crate::error::{Result, VaultdbError}` and `crate::record::{FieldValue, Record}`)
- `crates/vaultdb-cli/src/frontmatter.rs` (uses `crate::error` and `crate::record`)
- `crates/vaultdb-cli/src/links.rs` (uses `crate::record`)
- `crates/vaultdb-cli/src/output.rs` (uses `crate::record`)
- `crates/vaultdb-cli/src/schema.rs` (uses `crate::error` and `crate::record`)
- `crates/vaultdb-cli/src/vault.rs` (uses `crate::error` and `crate::record`)
- `crates/vaultdb-cli/src/writer.rs` (uses `crate::error`)
- `crates/vaultdb-cli/src/commands/*.rs` (several use `crate::error` and `crate::record`)

Do the replacement file-by-file, not with a blind sed: some files use multiple imports from the same module on one `use` line (e.g., `use crate::error::{Result, VaultdbError}` becomes `use vaultdb_core::error::{Result, VaultdbError}`).

Also note that `error.rs` defines a public `Result` type alias. Files that currently `use crate::error::Result;` keep that as `use vaultdb_core::error::Result;` — no behaviour change.

- [ ] **Step 6: Build the workspace**

```bash
cargo build --workspace
```

Expected: builds successfully. If you see errors like "unresolved import `crate::error`", you missed a file in step 5 — search again with `grep -rn "crate::error" crates/vaultdb-cli/src/` and fix.

- [ ] **Step 7: Run the test suite**

```bash
cargo test --workspace
```

Expected: all tests still pass (same count as in Task 2). The unit tests for `record` now run from inside `vaultdb-core`; the unit tests for everything else still run from inside `vaultdb-cli`.

- [ ] **Step 8: Commit**

```bash
git add -A
git status
git commit -m "refactor(core): move error and record modules to vaultdb-core

Moves the two no-internal-deps engine modules into vaultdb-core.
vaultdb-cli now depends on vaultdb-core via path dep and imports
\`vaultdb_core::error\` and \`vaultdb_core::record\` in place of
\`crate::error\` and \`crate::record\`. No behaviour change."
```

---

## Task 4: Move the next dependency layer (`frontmatter`, `links`, `filter`, `schema`)

**Goal:** Move the modules that depend only on `error` and `record` (already in `vaultdb-core`).

These four modules form the next layer:
- `frontmatter.rs` — depends on `error`, `record`
- `links.rs` — depends on `record`
- `filter.rs` — depends on `error`, `record`
- `schema.rs` — depends on `error`, `record`

After this task, the only engine module remaining in `vaultdb-cli/src/` is `vault.rs` and `writer.rs` (covered in task 5).

**Files:**
- Move: `crates/vaultdb-cli/src/{frontmatter,links,filter,schema}.rs` → `crates/vaultdb-core/src/`
- Modify: `crates/vaultdb-core/src/lib.rs`
- Modify: `crates/vaultdb-cli/src/main.rs`
- Modify: every consumer of those modules in `vaultdb-cli/src/`

- [ ] **Step 1: Move the files**

```bash
cd /home/rusen/Desktop/codebase-shared/researches/vaultdb
git mv crates/vaultdb-cli/src/frontmatter.rs crates/vaultdb-core/src/frontmatter.rs
git mv crates/vaultdb-cli/src/links.rs       crates/vaultdb-core/src/links.rs
git mv crates/vaultdb-cli/src/filter.rs      crates/vaultdb-core/src/filter.rs
git mv crates/vaultdb-cli/src/schema.rs      crates/vaultdb-core/src/schema.rs
```

- [ ] **Step 2: Expose them from `vaultdb-core`'s `lib.rs`**

Replace `crates/vaultdb-core/src/lib.rs` with:

```rust
//! vaultdb-core — library engine for vaultdb.

pub mod error;
pub mod filter;
pub mod frontmatter;
pub mod links;
pub mod record;
pub mod schema;
```

- [ ] **Step 3: Update intra-core imports**

The four modules just moved still reference `crate::error` and `crate::record`. Inside `vaultdb-core`, `crate::error` and `crate::record` are valid (they refer to `vaultdb-core`'s own modules). **No change needed here** — the imports work as-is.

Sanity-check by grepping inside the moved files:

```bash
grep -E "use crate::" crates/vaultdb-core/src/{frontmatter,links,filter,schema}.rs
```

Expected output (each `crate::` reference is to `error` or `record`, both of which are vaultdb-core modules — leave them alone):

```
crates/vaultdb-core/src/frontmatter.rs:use crate::error::{Result, VaultdbError};
crates/vaultdb-core/src/frontmatter.rs:use crate::record::{FieldValue, Record};
crates/vaultdb-core/src/links.rs:use crate::record::{FieldValue, Record};
crates/vaultdb-core/src/filter.rs:use crate::error::{Result, VaultdbError};
crates/vaultdb-core/src/filter.rs:use crate::record::{FieldValue, Record};
crates/vaultdb-core/src/schema.rs:use crate::error::{Result, VaultdbError};
crates/vaultdb-core/src/schema.rs:use crate::record::FieldValue;
```

If any line references `crate::frontmatter` or `crate::links` or `crate::filter` or `crate::schema`, those are also fine — they all live in vaultdb-core now.

- [ ] **Step 4: Drop the moved modules from `vaultdb-cli`'s `main.rs`**

Edit `crates/vaultdb-cli/src/main.rs`. The `mod` block currently reads:

```rust
mod cli;
mod commands;
mod filter;
mod frontmatter;
mod links;
mod output;
mod schema;
mod vault;
mod writer;
```

Drop the four moved modules:

```rust
mod cli;
mod commands;
mod output;
mod vault;
mod writer;
```

- [ ] **Step 5: Sweep-replace `crate::frontmatter`, `crate::links`, `crate::filter`, `crate::schema` in `vaultdb-cli`**

```bash
grep -rEn "crate::(frontmatter|links|filter|schema)" crates/vaultdb-cli/src/
```

For each match, change `crate::X` → `vaultdb_core::X`. Likely files needing edits (based on the dep map):
- `crates/vaultdb-cli/src/output.rs` (`crate::links::LinkIndex`)
- `crates/vaultdb-cli/src/vault.rs` (`crate::frontmatter`)
- `crates/vaultdb-cli/src/writer.rs` (likely uses `crate::frontmatter` for write-path parsing — verify)
- `crates/vaultdb-cli/src/commands/create.rs` (`crate::frontmatter`)
- `crates/vaultdb-cli/src/commands/update.rs` (`crate::filter::{WhereClause, matches_all}`)
- `crates/vaultdb-cli/src/commands/delete.rs` (`crate::filter`, `crate::links`)
- `crates/vaultdb-cli/src/commands/move_cmd.rs` (`crate::filter`)
- `crates/vaultdb-cli/src/commands/schema_cmd.rs` (`crate::filter`, `crate::schema`)
- `crates/vaultdb-cli/src/commands/links.rs` (`crate::links`)
- `crates/vaultdb-cli/src/commands/traverse.rs` (`crate::filter`, `crate::links`)
- `crates/vaultdb-cli/src/commands/unresolved.rs` (`crate::links`)
- `crates/vaultdb-cli/src/commands/rename.rs` (`crate::links`)
- `crates/vaultdb-cli/src/commands/query.rs` (`crate::filter`, `crate::links`)

- [ ] **Step 6: Build**

```bash
cargo build --workspace
```

Expected: builds successfully. Any unresolved imports point to a missed `crate::` reference; grep again and fix.

- [ ] **Step 7: Test**

```bash
cargo test --workspace
```

Expected: all tests pass. Test count should be unchanged from task 3 (tests moved with their modules; no tests added or removed).

- [ ] **Step 8: Commit**

```bash
git add -A
git status
git commit -m "refactor(core): move frontmatter, links, filter, schema to vaultdb-core

Moves the four engine modules whose only internal dependencies are on
error and record (already in vaultdb-core). All vaultdb-cli consumers
updated to import from vaultdb_core::*. No behaviour change."
```

---

## Task 5: Move the final engine modules (`vault`, `writer`)

**Goal:** Move the last two engine modules. After this, every file under `crates/vaultdb-cli/src/` is CLI-specific.

`vault.rs` depends on `error`, `frontmatter`, `record` — all in `vaultdb-core` already. `writer.rs` depends on `error` directly and likely on `record`/`frontmatter` indirectly.

**Files:**
- Move: `crates/vaultdb-cli/src/{vault,writer}.rs` → `crates/vaultdb-core/src/`
- Modify: `crates/vaultdb-core/src/lib.rs`
- Modify: `crates/vaultdb-cli/src/main.rs`
- Modify: every consumer of `vault` or `writer` in `vaultdb-cli/src/`

- [ ] **Step 1: Move the files**

```bash
cd /home/rusen/Desktop/codebase-shared/researches/vaultdb
git mv crates/vaultdb-cli/src/vault.rs  crates/vaultdb-core/src/vault.rs
git mv crates/vaultdb-cli/src/writer.rs crates/vaultdb-core/src/writer.rs
```

- [ ] **Step 2: Expose them from `vaultdb-core`'s `lib.rs`**

Replace `crates/vaultdb-core/src/lib.rs` with:

```rust
//! vaultdb-core — library engine for vaultdb.

pub mod error;
pub mod filter;
pub mod frontmatter;
pub mod links;
pub mod record;
pub mod schema;
pub mod vault;
pub mod writer;
```

- [ ] **Step 3: Drop `mod vault;` and `mod writer;` from `vaultdb-cli`'s `main.rs`**

Edit `crates/vaultdb-cli/src/main.rs`. The `mod` block currently reads:

```rust
mod cli;
mod commands;
mod output;
mod vault;
mod writer;
```

Drop the moved modules:

```rust
mod cli;
mod commands;
mod output;
```

Also: at the bottom of the same file there is:

```rust
use cli::{Cli, Command};
use vault::Vault;
```

Change to:

```rust
use cli::{Cli, Command};
use vaultdb_core::vault::Vault;
```

(Or add a `use vaultdb_core::Vault;` once we have a top-level re-export. For phase 1, going through the module path is fine. Phase 2 introduces top-level re-exports.)

- [ ] **Step 4: Sweep-replace `crate::vault` and `crate::writer` in `vaultdb-cli`**

```bash
grep -rEn "crate::(vault|writer)" crates/vaultdb-cli/src/
```

For each match, change to `vaultdb_core::vault` / `vaultdb_core::writer`. Likely files:
- Every `crates/vaultdb-cli/src/commands/*.rs` (most commands use `crate::vault::Vault`)
- `crates/vaultdb-cli/src/commands/create.rs`, `update.rs`, `delete.rs`, `rename.rs` use `crate::writer`

- [ ] **Step 5: Build**

```bash
cargo build --workspace
```

Expected: builds successfully. If errors mention `vault` or `writer`, grep again.

- [ ] **Step 6: Test**

```bash
cargo test --workspace
```

Expected: all tests pass. Same count as task 4.

- [ ] **Step 7: Verify `vaultdb-cli/src/` contains only CLI-specific code**

```bash
ls crates/vaultdb-cli/src/
```

Expected output:

```
cli.rs
commands
main.rs
output.rs
```

Plus the `commands/` directory with its 10 command files. **No engine modules** should remain.

```bash
ls crates/vaultdb-cli/src/commands/
```

Expected output:

```
create.rs
delete.rs
links.rs
mod.rs
move_cmd.rs
query.rs
rename.rs
schema_cmd.rs
traverse.rs
unresolved.rs
update.rs
```

- [ ] **Step 8: Verify `vaultdb-core/src/` contains exactly the engine modules**

```bash
ls crates/vaultdb-core/src/
```

Expected output:

```
error.rs
filter.rs
frontmatter.rs
lib.rs
links.rs
record.rs
schema.rs
vault.rs
writer.rs
```

- [ ] **Step 9: Commit**

```bash
git add -A
git status
git commit -m "refactor(core): move vault and writer to vaultdb-core; complete split

vault.rs and writer.rs were the last engine modules in vaultdb-cli.
After this commit, crates/vaultdb-cli/src/ contains only CLI code
(cli.rs, output.rs, main.rs, commands/) and crates/vaultdb-core/src/
contains the engine. No behaviour change."
```

---

## Task 6: Final verification

**Goal:** Confirm the split is complete and the binary behaves identically to pre-refactor.

- [ ] **Step 1: Run the full test suite**

```bash
cd /home/rusen/Desktop/codebase-shared/researches/vaultdb
cargo test --workspace
```

Expected: all tests pass. Count should match pre-refactor.

- [ ] **Step 2: Run clippy across the workspace**

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

Expected: no warnings. If clippy now flags previously-suppressed warnings (e.g., unused imports surfaced by the split), fix them. The `#![allow(dead_code)]` at the top of `main.rs` was suppressing legitimate dead-code warnings; with the split, much of the dead code is now in the library where it's used by future consumers, so clippy should be happier than before.

- [ ] **Step 3: Build release**

```bash
cargo build --release
```

Expected: produces `target/release/vaultdb`.

- [ ] **Step 4: Smoke-test commands against a real vault**

```bash
./target/release/vaultdb --help
```

Expected: shows the same help text as the pre-refactor version.

```bash
# Pick a vault you have locally — e.g., ~/Documents/rusen-brain
./target/release/vaultdb --vault ~/Documents/rusen-brain query 3-Notes --select "_name,_backlink_count" --sort _backlink_count --desc --limit 5
```

Expected: returns a comfy-table with 5 rows. Same output as pre-refactor.

```bash
./target/release/vaultdb --vault ~/Documents/rusen-brain fields 3-Notes
```

Expected: lists frontmatter fields with types and frequencies. Same output as pre-refactor.

- [ ] **Step 5: Verify the workspace cleanly handles a published-style install**

```bash
cargo install --path crates/vaultdb-cli --force
which vaultdb
vaultdb --help
```

Expected: installs to `~/.cargo/bin/vaultdb`; help text matches.

- [ ] **Step 6: Update the README's install instructions**

The current `README.md` says:

```bash
git clone https://github.com/rusenbb/vaultdb.git
cd vaultdb
cargo install --path .
```

The `cargo install --path .` form no longer works post-split (the root is a workspace, not a package). Update the README to:

```bash
git clone https://github.com/rusenbb/vaultdb.git
cd vaultdb
cargo install --path crates/vaultdb-cli
```

And update the "Or just build" section similarly:

```bash
cargo build --release
# Binary at target/release/vaultdb
```

(The `target/release/vaultdb` path is unchanged because of the `[[bin]] name = "vaultdb"` declaration in `crates/vaultdb-cli/Cargo.toml`.)

- [ ] **Step 7: Commit the README update**

```bash
git add README.md
git commit -m "docs: update install instructions for workspace layout

cargo install now points at crates/vaultdb-cli; the workspace root is
no longer an installable package. The produced binary is unchanged
(target/release/vaultdb)."
```

- [ ] **Step 8: Verify the commit history is clean**

```bash
git log --oneline -10
```

Expected: a series of commits, one per task, with descriptive messages. Something like:

```
xxxxxxx docs: update install instructions for workspace layout
xxxxxxx refactor(core): move vault and writer to vaultdb-core; complete split
xxxxxxx refactor(core): move frontmatter, links, filter, schema to vaultdb-core
xxxxxxx refactor(core): move error and record modules to vaultdb-core
xxxxxxx feat(workspace): scaffold empty vaultdb-core library crate
xxxxxxx refactor(workspace): move all source into crates/vaultdb-cli/
... (older history)
```

If any commit message is wrong or commits got bundled, that's a manual fix decision for the operator — do not rebase autonomously.

---

## Open questions / followups (not in scope for phase 1)

- **`#![allow(dead_code)]` in `main.rs`**: still present after this phase. With the split, much of the previously-dead code is genuinely *public* (re-exposed from `vaultdb-core` for future consumers). The flag may not be needed anymore. Phase 2 (API hardening) will address this when it defines `vaultdb-core`'s public surface and removes the flag.
- **Workspace shared dependencies**: this plan keeps each crate's `[dependencies]` independent. Phase 2 may introduce `[workspace.dependencies]` to centralise version pins (`serde = "1"` would live once at the workspace level, with `serde = { workspace = true }` in each member). Out of scope for phase 1 because it's an ergonomic improvement, not a behaviour change.
- **Shared `[workspace.package]` metadata inheritance**: for now each crate hard-codes `version`, `edition`, `license`, `repository`. Phase 2 may consolidate these via `[workspace.package]` + `version.workspace = true`. Same reason as above — out of scope.
- **`Cargo.lock`**: regenerated by Cargo on first build of the workspace. Commit the regenerated lockfile as part of task 1's commit. If Git shows a large lockfile diff, that's expected.
- **`vaultdb-mcp`**: this plan does *not* create the `vaultdb-mcp` crate. The workspace `members` list contains only `vaultdb-core` and `vaultdb-cli` after this phase. `vaultdb-mcp` arrives in phase 3.
