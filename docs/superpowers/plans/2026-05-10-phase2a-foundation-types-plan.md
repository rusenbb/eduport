# Phase 2a — vaultdb-core Foundation Types & Diagnostics — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Harden vaultdb-core's foundation types and diagnostics so future consumers (eduport-core, vaultdb-mcp, third parties) get a clean, stable, well-typed surface — without touching the query/mutation API yet.

**Architecture:** Pure additive + rename refactor inside vaultdb-core, plus housekeeping in vaultdb-cli's manifest. The bigger AST/builder/LinkGraph redesign is deferred to Phase 2b — this plan only makes the *value-shaped* parts of the public API ready.

**Tech Stack:** Rust 2024 edition, the existing vaultdb workspace (`vaultdb-core` library + `vaultdb` binary), `serde` and `serde_yaml`, no new crates.

**Working directory for all tasks:** `/home/rusen/Desktop/codebase-shared/researches/vaultdb/.worktrees/phase2a-foundation-types` (a git worktree on branch `feat/phase2a-foundation-types`, branched from `main` at SHA `abb2f21`). Set up before Task 1 begins. **All `cargo` and `git` commands run from there**, not from eduport's working tree.

**Baseline before Task 1:** `cargo test --workspace` reports 98 tests passing. The test count grows over the phase (each task that adds new tests notes the new expected total) — but the count must never *drop* below the prior task's expectation, and no existing test should fail. Final expected total at end of phase: 114.

**Out of scope for Phase 2a:**
- The new public `Expr`/`Predicate`/`LinkPredicate` query AST (Phase 2b).
- The `UpdateBuilder` / `DeleteBuilder` / `MoveBuilder` / `RenameBuilder` typed mutation API (Phase 2b).
- The public `LinkGraph` type with `GraphScope` (Phase 2b).
- Migrating CLI commands to use new builders (Phase 2b).
- crates.io republishing (you do this manually after both 2a and 2b ship).
- `[workspace.package]` field inheritance (defer; non-blocking).

---

## File structure after this phase

```
crates/vaultdb-core/src/
├── lib.rs              gains `pub use record::{Record, Value}`, `pub use vault::LoadResult`,
│                       `pub use error::ParseError`, plus crate-level rustdoc
├── error.rs            gains `pub struct ParseError { file, message }` alongside
│                       the existing `VaultdbError` enum
├── record.rs           `FieldValue` renamed to `Value`; gains as_*/is_* helpers,
│                       Serialize/Deserialize derives
├── vault.rs            `load_records` returns `LoadResult { records, parse_errors }`;
│                       new `find_by_name` method
├── frontmatter.rs      `FieldValue` → `Value` (mechanical rename)
├── filter.rs           `FieldValue` → `Value` (mechanical rename)
├── links.rs            `FieldValue` → `Value` (mechanical rename)
├── schema.rs           `FieldValue` → `Value`; `FieldSchema::enum_values: Vec<serde_yaml::Value>`
│                       changes to `Vec<Value>`
└── writer.rs           unchanged in 2a (no FieldValue usage there)

crates/vaultdb/src/
├── main.rs             drops `#![allow(dead_code)]`
├── output.rs           `FieldValue` → `Value` (mechanical rename)
├── commands/
│   ├── query.rs        `FieldValue` → `Value` (mechanical rename)
│   ├── schema_cmd.rs   pattern matches on `serde_yaml::Value` change to `Value`
│   └── (others)        unchanged
└── (Cargo.toml)        drop direct deps `walkdir`, `regex`, `serde_yaml` (transitive via vaultdb-core)
```

Total files touched: ~13.

---

## Task 1: Set up the Phase 2a worktree

**Goal:** Create the isolated worktree on a fresh feature branch off main. Verify baseline.

**Files:** none (workspace setup only).

- [ ] **Step 1: Verify main is up to date and clean**

```bash
cd /home/rusen/Desktop/codebase-shared/researches/vaultdb
git status
git log --oneline -3
```

Expected: clean working tree, on `main`, latest commit is `abb2f21 refactor(workspace): rename binary crate from vaultdb-cli to vaultdb`.

If the branch is dirty, stop and report.

- [ ] **Step 2: Create the worktree on a new feature branch**

```bash
git worktree add .worktrees/phase2a-foundation-types -b feat/phase2a-foundation-types
cd .worktrees/phase2a-foundation-types
git status
git branch --show-current
```

Expected: clean working tree, branch `feat/phase2a-foundation-types`.

- [ ] **Step 3: Verify the build and test baseline**

```bash
cargo build --workspace 2>&1 | tail -3
cargo test --workspace 2>&1 | grep -E "^test result:" | awk -F'[ ;]' '{sum+=$4} END {print "Total tests passing:", sum}'
```

Expected: build succeeds, output shows `Total tests passing: 98`.

If the count differs from 98, stop — something is wrong with the baseline before any work begins.

- [ ] **Step 4: No commit — this task is workspace setup only.** Do not commit anything. Proceed to Task 2.

---

## Task 2: Rename `FieldValue` to `Value` and add convenience helpers + serde derives

**Goal:** Rename the value type to `Value` (the public name from spec section 5) across the codebase. Add `as_*` / `is_*` accessors and `Serialize` / `Deserialize` derives to make `Value` library-friendly.

**Files:**
- Modify: `crates/vaultdb-core/src/record.rs` (rename + helpers + derives)
- Modify (mechanical rename, no logic changes): `crates/vaultdb-core/src/{filter,frontmatter,links,schema}.rs`, `crates/vaultdb/src/output.rs`, `crates/vaultdb/src/commands/query.rs`

- [ ] **Step 1: Bulk-rename `FieldValue` → `Value` across the workspace**

```bash
cd /home/rusen/Desktop/codebase-shared/researches/vaultdb/.worktrees/phase2a-foundation-types
grep -rln "FieldValue" --include="*.rs"
# Expected output: 7 files (verify before next command)

# Use sed to do the rename. Note: `Value` is generic enough that we use word
# boundaries to avoid collisions. Rust word boundaries with sed are POSIX-class.
find crates -type f -name "*.rs" -exec sed -i 's/\bFieldValue\b/Value/g' {} +

# Verify no FieldValue references remain
grep -rn "FieldValue" --include="*.rs" || echo "no matches — rename complete"
```

Expected: the find/grep at the end prints `no matches — rename complete`.

Sanity-check that the rename didn't accidentally collide with another type called `Value`:

```bash
grep -rn "use serde_yaml::Value\b" --include="*.rs"
```

Expected output: matches in `crates/vaultdb-core/src/schema.rs` and `crates/vaultdb/src/commands/schema_cmd.rs`. These imports use `serde_yaml::Value` — fully qualified, so the bare-`Value` rename does not collide. (Task 5 will replace these with our new `Value` anyway.)

- [ ] **Step 2: Add convenience helpers and serde derives to `Value`**

Open `crates/vaultdb-core/src/record.rs`. The existing `Value` enum (formerly `FieldValue`) currently derives `Debug, Clone, PartialEq` only. Update it to:

```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum Value {
    Null,
    String(String),
    Integer(i64),
    Float(f64),
    Bool(bool),
    List(Vec<Value>),
    Map(std::collections::BTreeMap<String, Value>),
}
```

The `#[serde(untagged)]` attribute makes `Value` (de)serialize as the natural JSON/YAML scalar/list/map shape, not as a tagged enum. This is what consumers expect.

Then add the helper methods immediately AFTER the existing `impl Value { ... }` block (or inside it, depending on where the existing impl is — keep all `impl Value` blocks together):

```rust
impl Value {
    /// Returns the inner string if this is `Value::String`, else `None`.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the inner integer if this is `Value::Integer`, else `None`.
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            Value::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// Returns the inner float if this is `Value::Float`, else `None`.
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// Returns the inner bool if this is `Value::Bool`, else `None`.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Returns the inner list if this is `Value::List`, else `None`.
    pub fn as_list(&self) -> Option<&[Value]> {
        match self {
            Value::List(v) => Some(v),
            _ => None,
        }
    }

    /// Returns the inner map if this is `Value::Map`, else `None`.
    pub fn as_map(&self) -> Option<&std::collections::BTreeMap<String, Value>> {
        match self {
            Value::Map(m) => Some(m),
            _ => None,
        }
    }

    /// Returns true if this value is `Value::Null`.
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }
}
```

Note: do NOT remove existing methods on `Value` — only ADD the helpers. If the existing code already has any of the above methods (unlikely), skip the duplicate.

- [ ] **Step 3: Add unit tests for the helpers**

Inside `crates/vaultdb-core/src/record.rs`, locate the existing `#[cfg(test)] mod tests { ... }` block. Add these test functions inside it (alongside the existing tests, do not replace them):

```rust
#[test]
fn value_helpers_string() {
    let v = Value::String("hi".into());
    assert_eq!(v.as_str(), Some("hi"));
    assert_eq!(v.as_integer(), None);
    assert!(!v.is_null());
}

#[test]
fn value_helpers_integer() {
    let v = Value::Integer(7);
    assert_eq!(v.as_integer(), Some(7));
    assert_eq!(v.as_float(), None);
    assert!(!v.is_null());
}

#[test]
fn value_helpers_float() {
    let v = Value::Float(1.5);
    assert_eq!(v.as_float(), Some(1.5));
}

#[test]
fn value_helpers_bool() {
    let v = Value::Bool(true);
    assert_eq!(v.as_bool(), Some(true));
}

#[test]
fn value_helpers_list() {
    let v = Value::List(vec![Value::Integer(1), Value::Integer(2)]);
    assert_eq!(v.as_list().map(|s| s.len()), Some(2));
}

#[test]
fn value_helpers_map() {
    let mut m = std::collections::BTreeMap::new();
    m.insert("k".into(), Value::String("v".into()));
    let v = Value::Map(m);
    assert_eq!(v.as_map().map(|m| m.len()), Some(1));
}

#[test]
fn value_helpers_null() {
    let v = Value::Null;
    assert!(v.is_null());
    assert_eq!(v.as_str(), None);
}

#[test]
fn value_serializes_untagged() {
    let v = Value::List(vec![Value::Integer(1), Value::String("x".into())]);
    let json = serde_json::to_string(&v).unwrap();
    assert_eq!(json, r#"[1,"x"]"#);
}

#[test]
fn value_deserializes_untagged() {
    let v: Value = serde_json::from_str(r#"[1,"x"]"#).unwrap();
    assert_eq!(
        v,
        Value::List(vec![Value::Integer(1), Value::String("x".into())])
    );
}
```

- [ ] **Step 4: Re-export `Value` and `Record` from `lib.rs`**

Open `crates/vaultdb-core/src/lib.rs`. Currently it contains:

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

Replace with (added: `pub use record::{Record, Value};`):

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

pub use record::{Record, Value};
```

- [ ] **Step 5: Build the workspace**

```bash
cargo build --workspace
```

Expected: builds cleanly. If you see "cannot find type `FieldValue`" anywhere, the rename missed a file — re-run `grep -rn "FieldValue" --include="*.rs"` and fix.

- [ ] **Step 6: Run the test suite**

```bash
cargo test --workspace 2>&1 | grep -E "^test result:" | awk -F'[ ;]' '{sum+=$4} END {print "Total:", sum}'
```

Expected: `Total: 107` (98 baseline + 9 new value-helper tests added in Step 3).

If you see `108` or higher, you accidentally added an extra test. If you see `106` or lower, one of the new tests didn't run — re-check Step 3.

- [ ] **Step 7: Commit**

```bash
git add -A
git status
git commit -m "$(cat <<'EOF'
refactor(core): rename FieldValue to Value, add helpers + serde derives

Renames the core value type to its public name from the spec.
Adds as_str / as_integer / as_float / as_bool / as_list / as_map /
is_null convenience accessors, plus #[derive(Serialize, Deserialize)]
with #[serde(untagged)] for natural scalar/list/map (de)serialization.

Re-exports Record and Value from lib.rs for the public API.

170 references renamed across 7 files (mechanical sed). 9 new unit
tests; total now 107 (was 98).
EOF
)"
```

---

## Task 3: Add `pub struct ParseError` to `vaultdb-core`'s error module

**Goal:** Introduce a public `ParseError` struct that the upcoming `LoadResult` (Task 4) will use to surface per-file parse failures as first-class results, instead of silently skipping them.

**Files:**
- Modify: `crates/vaultdb-core/src/error.rs` (add the struct alongside the existing `VaultdbError`)
- Modify: `crates/vaultdb-core/src/lib.rs` (re-export `ParseError`)

- [ ] **Step 1: Add `ParseError` to `error.rs`**

Open `crates/vaultdb-core/src/error.rs`. At the top of the file, add (after the existing `use` statements but before the `VaultdbError` enum):

```rust
use std::path::PathBuf;

/// A non-fatal parse failure for a single file.
///
/// Returned in `LoadResult::parse_errors` when `Vault::load_records` encounters
/// a file with malformed frontmatter. The application layer decides whether to
/// surface, log, or ignore these.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ParseError {
    pub file: PathBuf,
    pub message: String,
}
```

If `use std::path::PathBuf;` already exists at the top of the file, do not duplicate it. (Likely it does not — check first.)

- [ ] **Step 2: Re-export `ParseError` from `lib.rs`**

Open `crates/vaultdb-core/src/lib.rs`. Append to the bottom:

```rust
pub use error::ParseError;
```

The full `lib.rs` should now read:

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

pub use record::{Record, Value};
pub use error::ParseError;
```

- [ ] **Step 3: Add a unit test for round-trip serialization**

In `crates/vaultdb-core/src/error.rs`, append at the bottom (or inside an existing `#[cfg(test)]` block if there is one — check first):

```rust
#[cfg(test)]
mod parse_error_tests {
    use super::ParseError;
    use std::path::PathBuf;

    #[test]
    fn parse_error_serializes() {
        let err = ParseError {
            file: PathBuf::from("foo.md"),
            message: "bad yaml".into(),
        };
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("foo.md"));
        assert!(json.contains("bad yaml"));
    }

    #[test]
    fn parse_error_round_trips() {
        let err = ParseError {
            file: PathBuf::from("notes/x.md"),
            message: "oops".into(),
        };
        let json = serde_json::to_string(&err).unwrap();
        let back: ParseError = serde_json::from_str(&json).unwrap();
        assert_eq!(back.file, err.file);
        assert_eq!(back.message, err.message);
    }
}
```

- [ ] **Step 4: Build and test**

```bash
cargo build --workspace
cargo test --workspace 2>&1 | grep -E "^test result:" | awk -F'[ ;]' '{sum+=$4} END {print "Total:", sum}'
```

Expected: build clean, `Total: 109` (107 from Task 2 + 2 new tests).

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat(core): add public ParseError struct for parse diagnostics

Introduces ParseError { file: PathBuf, message: String } alongside
VaultdbError. ParseError is for non-fatal per-file parse failures
(currently surfaced through LoadResult.parse_errors in Task 4 next).
Derives Serialize/Deserialize so consumers can wire it through IPC
(Tauri commands, MCP wire format, HTTP) without conversion.
EOF
)"
```

---

## Task 4: Add `LoadResult` and refactor `Vault::load_records` to surface parse errors

**Goal:** Change `Vault::load_records` from `Result<Vec<Record>, VaultdbError>` to `Result<LoadResult, VaultdbError>`. Stop silently dropping files with invalid frontmatter — collect them as `parse_errors` instead.

**Files:**
- Modify: `crates/vaultdb-core/src/vault.rs` (define `LoadResult`, refactor `load_records`)
- Modify: `crates/vaultdb-core/src/lib.rs` (re-export `LoadResult`)
- Modify: `crates/vaultdb/src/commands/{query,update,delete,move_cmd,rename,traverse,unresolved,links,schema_cmd,create}.rs` (callers — update to handle new return shape)

- [ ] **Step 1: Define `LoadResult` in `vault.rs`**

Open `crates/vaultdb-core/src/vault.rs`. Near the top of the file (after existing `use` statements), add:

```rust
/// Records loaded from a folder, with per-file parse diagnostics.
///
/// Files with malformed YAML frontmatter appear in `parse_errors` rather than
/// being silently dropped. Files without frontmatter at all are loaded as
/// empty records (this is intentional — they remain queryable by virtual
/// fields like `_name` / `_path`).
#[derive(Debug, Clone)]
pub struct LoadResult {
    pub records: Vec<Record>,
    pub parse_errors: Vec<crate::error::ParseError>,
}
```

(Don't derive Serialize/Deserialize here yet — `Record` itself doesn't yet implement them. We can add to both in a later phase.)

- [ ] **Step 2: Refactor `Vault::load_records` to return `LoadResult`**

In the same file, find the existing implementation of `Vault::load_records`. It currently returns `Result<Vec<Record>, VaultdbError>` and silently drops files on `InvalidFrontmatter` (logging only if `verbose` is set). Replace it with:

```rust
/// Load records from a folder, collecting per-file parse diagnostics.
///
/// Files with no frontmatter are loaded as empty records (queryable via
/// virtual fields). Files with invalid frontmatter are collected into
/// `LoadResult.parse_errors` rather than dropped.
///
/// `verbose` is preserved for compatibility with the CLI's `-v` flag — it
/// causes parse errors to also be logged to stderr as they're encountered.
/// Library consumers that don't want stderr logging should pass `false` and
/// inspect `parse_errors` themselves.
pub fn load_records(
    &self,
    folder: &Path,
    recursive: bool,
    verbose: bool,
) -> Result<LoadResult, VaultdbError> {
    let files = self.list_files(folder, recursive)?;
    let mut records = Vec::new();
    let mut parse_errors = Vec::new();

    for path in files {
        match frontmatter::load_record(&path) {
            Ok(record) => records.push(record),
            Err(VaultdbError::NoFrontmatter(_)) => {
                records.push(Record {
                    path: path.clone(),
                    fields: std::collections::BTreeMap::new(),
                    raw_content: None,
                });
            }
            Err(VaultdbError::InvalidFrontmatter { file, reason }) => {
                if verbose {
                    eprintln!("skipping (invalid frontmatter): {}: {}", file, reason);
                }
                parse_errors.push(crate::error::ParseError {
                    file: std::path::PathBuf::from(&file),
                    message: reason,
                });
            }
            Err(e) => return Err(e),
        }
    }

    Ok(LoadResult { records, parse_errors })
}
```

The companion `load_records_with_content` (which preserves raw file content for write operations) gets the same treatment. Find that function and rewrite it analogously, returning `LoadResult` and pushing into `parse_errors` instead of silently dropping. Match the structure of `load_records` above.

- [ ] **Step 3: Re-export `LoadResult` from `lib.rs`**

Open `crates/vaultdb-core/src/lib.rs`. Append:

```rust
pub use vault::{LoadResult, Vault};
```

If `pub use vault::Vault;` is already there alone, replace it with the combined form. The full `lib.rs` should now end with:

```rust
pub use record::{Record, Value};
pub use error::ParseError;
pub use vault::{LoadResult, Vault};
```

- [ ] **Step 4: Update CLI callers — extract `.records` from the new return type**

Every CLI command that called `vault.load_records(...)` or `vault.load_records_with_content(...)` and used the resulting `Vec<Record>` directly now needs to extract `.records`. Find them:

```bash
cd /home/rusen/Desktop/codebase-shared/researches/vaultdb/.worktrees/phase2a-foundation-types
grep -rn "load_records\|load_records_with_content" crates/vaultdb/src/
```

For EACH match in `crates/vaultdb/src/commands/*.rs` where the return value is bound, the change is mechanical:

**Before:**
```rust
let records = vault.load_records(&folder, recursive, verbose)?;
// ... use records as Vec<Record>
```

**After:**
```rust
let result = vault.load_records(&folder, recursive, verbose)?;
let records = result.records;
// At verbose mode, print parse errors. The eprintln in load_records still
// runs when verbose is true, so for now just keep records-only behaviour;
// the CLI's verbose output is unchanged.
// ... use records as Vec<Record>
```

Or more concisely:

**After (concise):**
```rust
let records = vault.load_records(&folder, recursive, verbose)?.records;
```

Use the concise form unless a specific command also needs to act on `parse_errors` (none do in this phase — the CLI continues to log via the `verbose` flag's `eprintln!`).

The same pattern applies for `load_records_with_content`. If a command also uses it, do the same `.records` extraction.

- [ ] **Step 5: Build the workspace**

```bash
cargo build --workspace
```

Expected: builds cleanly. If you see "expected `Vec<Record>`, found `LoadResult`" in any command file, you missed updating that caller.

- [ ] **Step 6: Run tests**

```bash
cargo test --workspace 2>&1 | grep -E "^test result:" | awk -F'[ ;]' '{sum+=$4} END {print "Total:", sum}'
```

Expected: `Total: 109` (no new tests in this task; existing tests still pass with the refactored `load_records` returning the new shape — the inline tests in `vault.rs` need updating too).

If a test fails because it expected `Vec<Record>` and got `LoadResult`, update the test inline (just add `.records` after the call). Inside `vault.rs`'s `#[cfg(test)] mod tests { ... }` block, find any `vault.load_records(...)` calls and append `.records` to extract the records.

After the test fixes, re-run `cargo test --workspace`. Expected: `Total: 109` and all green.

- [ ] **Step 7: Add a new test for parse-error surfacing**

Inside `crates/vaultdb-core/src/vault.rs`'s `#[cfg(test)] mod tests`, add:

```rust
#[test]
fn load_records_surfaces_invalid_frontmatter_as_parse_errors() {
    use std::fs;

    let dir = create_test_vault();
    // Add a file with malformed YAML frontmatter
    fs::write(
        dir.path().join("notes/broken.md"),
        "---\n: : : not yaml\n---\nbody\n",
    )
    .unwrap();

    let vault = Vault::with_root(dir.path().to_path_buf());
    let result = vault
        .load_records(&dir.path().join("notes"), false, false)
        .unwrap();

    // The 3 valid-or-empty files (test1, test2, no_fm) load as records;
    // broken.md is collected as a parse error.
    assert_eq!(result.records.len(), 3);
    assert_eq!(result.parse_errors.len(), 1);
    assert!(result.parse_errors[0].file.ends_with("broken.md"));
    assert!(!result.parse_errors[0].message.is_empty());
}
```

(`create_test_vault` is the existing helper at the top of the `mod tests` block. Reuse it.)

- [ ] **Step 8: Test again**

```bash
cargo test --workspace 2>&1 | grep -E "^test result:" | awk -F'[ ;]' '{sum+=$4} END {print "Total:", sum}'
```

Expected: `Total: 110` (109 previously + 1 new diagnostic test).

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat(core): LoadResult exposes parse errors instead of silently dropping

Vault::load_records now returns Result<LoadResult, VaultdbError>, where
LoadResult has both records (Vec<Record>) and parse_errors
(Vec<ParseError>). Files with invalid frontmatter no longer silently
disappear — they're surfaced via parse_errors so consumers (eduport,
mcp, etc) can prompt the user to fix them.

The CLI's verbose flag still logs errors to stderr in the same shape
as before. Same for load_records_with_content.

Adds one diagnostic test; total tests 110 (was 109).
EOF
)"
```

---

## Task 5: Add `Vault::find_by_name` for single-record lookup

**Goal:** Add a fast-path single-record lookup so library consumers (eduport, mcp) don't have to fold a folder scan + filter for "give me this one record".

**Files:**
- Modify: `crates/vaultdb-core/src/vault.rs` (add `find_by_name`)

- [ ] **Step 1: Add the method to `Vault`**

Open `crates/vaultdb-core/src/vault.rs`. Inside the existing `impl Vault { ... }` block, add (alongside the other methods):

```rust
/// Look up a single record by its filename (without the `.md` extension)
/// inside the given folder.
///
/// Returns `Ok(None)` if no such file exists. Returns `Ok(Some(record))`
/// when the file exists and parses cleanly. Returns
/// `Err(VaultdbError::InvalidFrontmatter)` if the file exists but its
/// frontmatter is malformed — unlike `load_records`, single-record lookup
/// surfaces parse errors as a hard error because the caller asked for one
/// specific record.
pub fn find_by_name(
    &self,
    folder: &str,
    name: &str,
) -> Result<Option<Record>, VaultdbError> {
    let folder_path = self.resolve_folder(folder)?;
    let candidate = folder_path.join(format!("{}.md", name));
    if !candidate.is_file() {
        return Ok(None);
    }
    match frontmatter::load_record(&candidate) {
        Ok(record) => Ok(Some(record)),
        Err(VaultdbError::NoFrontmatter(_)) => Ok(Some(Record {
            path: candidate,
            fields: std::collections::BTreeMap::new(),
            raw_content: None,
        })),
        Err(e) => Err(e),
    }
}
```

- [ ] **Step 2: Add unit tests**

In the same file, inside `#[cfg(test)] mod tests`, add:

```rust
#[test]
fn find_by_name_existing() {
    let dir = create_test_vault();
    let vault = Vault::with_root(dir.path().to_path_buf());
    let r = vault.find_by_name("notes", "test1").unwrap();
    assert!(r.is_some());
    assert_eq!(r.unwrap().virtual_name(), "test1");
}

#[test]
fn find_by_name_missing() {
    let dir = create_test_vault();
    let vault = Vault::with_root(dir.path().to_path_buf());
    let r = vault.find_by_name("notes", "no-such-record").unwrap();
    assert!(r.is_none());
}

#[test]
fn find_by_name_no_frontmatter_loads_as_empty() {
    let dir = create_test_vault();
    let vault = Vault::with_root(dir.path().to_path_buf());
    // create_test_vault() writes notes/no_fm.md with no frontmatter
    let r = vault.find_by_name("notes", "no_fm").unwrap().unwrap();
    assert!(r.fields.is_empty());
    assert_eq!(r.virtual_name(), "no_fm");
}

#[test]
fn find_by_name_invalid_frontmatter_errors() {
    use std::fs;
    let dir = create_test_vault();
    fs::write(
        dir.path().join("notes/broken.md"),
        "---\n: : :\n---\n",
    )
    .unwrap();
    let vault = Vault::with_root(dir.path().to_path_buf());
    let result = vault.find_by_name("notes", "broken");
    assert!(matches!(
        result,
        Err(VaultdbError::InvalidFrontmatter { .. })
    ));
}
```

- [ ] **Step 3: Build and test**

```bash
cargo build --workspace
cargo test --workspace 2>&1 | grep -E "^test result:" | awk -F'[ ;]' '{sum+=$4} END {print "Total:", sum}'
```

Expected: `Total: 114` (110 + 4 new tests).

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat(core): add Vault::find_by_name for single-record lookup

Lets consumers fetch a record by filename without scanning a whole
folder. Useful for app layers that hold a stable filename → record
mapping (eduport's <slug>-<id>.md convention; mcp's "show me this
note" tools).

Returns Ok(None) if missing; Ok(Some(record)) if found; surfaces
InvalidFrontmatter as Err since the caller asked for one specific
file. 4 new unit tests; total 114.
EOF
)"
```

---

## Task 6: Replace `Vec<serde_yaml::Value>` in `FieldSchema::enum_values` with `Vec<Value>`

**Goal:** Eliminate the `serde_yaml::Value` leak in vaultdb-core's public API surface (flagged by Phase 1's final review). With `FieldSchema::enum_values` using our own `Value`, downstream consumers don't have to depend on `serde_yaml` to handle schema enum values.

**Files:**
- Modify: `crates/vaultdb-core/src/schema.rs` (change `enum_values` field type; update construction sites)
- Modify: `crates/vaultdb/src/commands/schema_cmd.rs` (pattern matches change from `serde_yaml::Value` to `Value`)

- [ ] **Step 1: Inspect the current shape**

```bash
cd /home/rusen/Desktop/codebase-shared/researches/vaultdb/.worktrees/phase2a-foundation-types
grep -n "enum_values" crates/vaultdb-core/src/schema.rs crates/vaultdb/src/commands/schema_cmd.rs
```

Read the matching lines so you know what construction sites you'll need to update. Specifically:
- In `schema.rs`: the `FieldSchema` struct definition has `enum_values: Vec<serde_yaml::Value>`. This is populated in the schema-inference logic (a function that walks records and collects unique values per field).
- In `schema_cmd.rs`: when displaying or validating, the code pattern-matches on `serde_yaml::Value::String(s)`, `serde_yaml::Value::Number(n)`, etc.

- [ ] **Step 2: Change the field type in `FieldSchema`**

Open `crates/vaultdb-core/src/schema.rs`. Find the `FieldSchema` struct definition. Change `enum_values: Vec<serde_yaml::Value>` to `enum_values: Vec<Value>`. (`Value` is the type from the same crate, accessible as `crate::record::Value` or `crate::Value` at this point — pick the one that matches the existing import style in the file.)

- [ ] **Step 3: Update the construction site(s) in `schema.rs`**

Find every place in `schema.rs` that pushes into `enum_values`. The values being pushed are `FieldValue` (now `Value`) extracted from records — they should already be in the right shape. The change is just removing the `serde_yaml::Value`-conversion that may have been there.

If the existing code does something like:

```rust
let yaml_v = serde_yaml::to_value(&field_value).unwrap();
schema.enum_values.push(yaml_v);
```

Replace with:

```rust
schema.enum_values.push(field_value.clone());
```

(or whatever ownership shape fits — read the surrounding code).

If the existing construction was already `enum_values.push(...)` with a `Value`-typed argument and only the field declaration was wrong, the change in Step 2 alone may be sufficient.

- [ ] **Step 4: Update `schema_cmd.rs` pattern matches**

Open `crates/vaultdb/src/commands/schema_cmd.rs`. Find every `match` arm or pattern that destructures `serde_yaml::Value::String(...)`, `serde_yaml::Value::Number(...)`, `serde_yaml::Value::Bool(...)`, etc. Each becomes the corresponding `Value::*` variant:

| Before | After |
|---|---|
| `serde_yaml::Value::String(s) => ...` | `Value::String(s) => ...` |
| `serde_yaml::Value::Number(n) if n.is_i64() => ...` | `Value::Integer(i) => ...` |
| `serde_yaml::Value::Number(n) if n.is_f64() => ...` | `Value::Float(f) => ...` |
| `serde_yaml::Value::Bool(b) => ...` | `Value::Bool(b) => ...` |
| `serde_yaml::Value::Null => ...` | `Value::Null => ...` |
| `serde_yaml::Value::Sequence(s) => ...` | `Value::List(s) => ...` |
| `serde_yaml::Value::Mapping(m) => ...` | `Value::Map(m) => ...` |

Note: `Value::Integer` and `Value::Float` are separate variants (vs `serde_yaml::Value::Number` which contains either). The match arms become more direct.

Also remove the `use serde_yaml::Value as YamlValue;` (or whatever alias is used) at the top of the file. Add `use vaultdb_core::Value;` if it's not already imported. Verify imports are clean.

- [ ] **Step 5: Build the workspace**

```bash
cargo build --workspace 2>&1 | tail -20
```

Expected: builds cleanly. If a pattern arm triggers `error[E0532]: expected tuple struct or tuple variant, found...`, you missed a substitution — search for remaining `serde_yaml::Value::` references in `schema_cmd.rs` and `schema.rs`:

```bash
grep -rn "serde_yaml::Value::" crates/
```

(There may be other valid uses of `serde_yaml::Value` elsewhere in the codebase — frontmatter.rs probably has them as part of YAML parsing. Those stay; only the schema-related ones are migrating.)

- [ ] **Step 6: Run tests**

```bash
cargo test --workspace 2>&1 | grep -E "^test result:" | awk -F'[ ;]' '{sum+=$4} END {print "Total:", sum}'
```

Expected: `Total: 114` (same as Task 5; this task changes types, doesn't add tests). Existing tests still pass.

If a schema test fails because it expected a `serde_yaml::Value` and now sees `Value`, update the test inline.

- [ ] **Step 7: Smoke-test the schema CLI**

```bash
# Use a tempdir vault to verify schema infer works end-to-end
tmpdir=$(mktemp -d)
mkdir -p "$tmpdir/.obsidian" "$tmpdir/notes"
cat > "$tmpdir/notes/a.md" <<'EOF'
---
status: active
priority: 1
---
body
EOF
cat > "$tmpdir/notes/b.md" <<'EOF'
---
status: pending
priority: 2
---
body
EOF

./target/debug/vaultdb --vault "$tmpdir" schema init notes
cat "$tmpdir/vaultdb-schema.yaml"
rm -rf "$tmpdir"
```

Expected: `schema init` runs cleanly; the generated `vaultdb-schema.yaml` contains correct field-type inference.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
refactor(core): use Value instead of serde_yaml::Value in FieldSchema

FieldSchema::enum_values is now Vec<Value> instead of
Vec<serde_yaml::Value>. This removes the serde_yaml leak from
vaultdb-core's public API — consumers no longer need to depend on
serde_yaml just to inspect schema enum values.

CLI's schema_cmd.rs pattern matches updated from serde_yaml::Value::*
to Value::* variants. The serde_yaml dep can be dropped from the
vaultdb crate's direct deps in the next task (it remains transitive
via vaultdb-core for frontmatter parsing).
EOF
)"
```

---

## Task 7: Drop redundant direct dependencies from the `vaultdb` binary crate's manifest

**Goal:** Remove `walkdir`, `regex`, and `serde_yaml` from `crates/vaultdb/Cargo.toml`'s `[dependencies]`. They were direct deps for historical reasons but are now used only transitively via `vaultdb-core` (or no longer used at all in CLI source).

**Files:**
- Modify: `crates/vaultdb/Cargo.toml`

- [ ] **Step 1: Verify the deps are unused in `vaultdb/src/`**

```bash
cd /home/rusen/Desktop/codebase-shared/researches/vaultdb/.worktrees/phase2a-foundation-types
grep -rn "use walkdir\|walkdir::" crates/vaultdb/src/
grep -rn "use regex\|regex::" crates/vaultdb/src/
grep -rn "use serde_yaml\|serde_yaml::" crates/vaultdb/src/
```

Expected after Task 6: each of these returns ZERO matches. If any return matches, the dep is still used directly; do NOT remove it.

- [ ] **Step 2: Edit `crates/vaultdb/Cargo.toml`**

Open the file. The current `[dependencies]` block:

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

Remove `serde_yaml = "0.9"`, `regex = "1"`, and `walkdir = "2"`. The block becomes:

```toml
[dependencies]
vaultdb-core = { path = "../vaultdb-core" }
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
comfy-table = "7"
colored = "2"
anyhow = "1"
thiserror = "2"
csv = "1"
```

Leave `[dev-dependencies]` unchanged.

- [ ] **Step 3: Build the workspace**

```bash
cargo build --workspace 2>&1 | tail -5
```

Expected: builds cleanly. If you see `unresolved import 'serde_yaml'` or similar in the CLI crate, the dep was actually still used; restore it and re-investigate Step 1.

- [ ] **Step 4: Run tests**

```bash
cargo test --workspace 2>&1 | grep -E "^test result:" | awk -F'[ ;]' '{sum+=$4} END {print "Total:", sum}'
```

Expected: `Total: 114`. No changes from Task 6.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
chore(workspace): drop redundant direct deps from vaultdb crate

After Task 6 removed the serde_yaml::Value pattern matches in
schema_cmd.rs, three deps in crates/vaultdb/Cargo.toml are now unused
in the CLI source: serde_yaml, regex, walkdir. They remain transitive
via vaultdb-core, which is the correct shape — declaring them
directly was misleading about what code in this crate actually uses.

Resolves a Phase 1 deferred item.
EOF
)"
```

---

## Task 8: Drop `#![allow(dead_code)]` from `main.rs` and resolve any resulting warnings

**Goal:** Remove the crate-level dead-code suppressor in the CLI binary. With the engine now in a separate library, dead-code warnings will surface only on items that are genuinely unused and should either be made `pub(crate)` (for library helpers) or removed.

**Files:**
- Modify: `crates/vaultdb/src/main.rs`
- Possibly modify: any file flagged as having dead code (likely none after the workspace split)

- [ ] **Step 1: Remove the attribute**

Open `crates/vaultdb/src/main.rs`. The first line is:

```rust
#![allow(dead_code)]
```

Delete that line and the (probably empty) line after it, so the file starts with the `mod` declarations.

- [ ] **Step 2: Build with warnings as errors**

```bash
cd /home/rusen/Desktop/codebase-shared/researches/vaultdb/.worktrees/phase2a-foundation-types
cargo build --workspace 2>&1 | tail -30
```

Expected outcomes:
- **Best case:** builds cleanly, no warnings. Move to Step 3.
- **Likely case:** one or two `dead_code` warnings on items in `vaultdb/src/` that the CLI doesn't actually use. For each warning:
  - If the item is genuinely unused (no callers in the CLI, no future plan to use it), DELETE it.
  - If it's a helper that *should* exist for the CLI but isn't currently called (e.g., a utility function added defensively), leave it but add `#[allow(dead_code)]` on the specific item with a one-line comment explaining why.
  - If you can't tell, ask the controller before either deleting or suppressing.

If the warning is in `vaultdb-core`, that's a separate question — `vaultdb-core` exposes `pub` items that are genuinely unused by `vaultdb-cli` but should remain for future consumers. Those won't trigger `dead_code` because `pub` items are exempt.

- [ ] **Step 3: Run clippy with `-D warnings`**

```bash
cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -20
```

If this fails because of the 7 pre-existing clippy lints (collapsible_if, needless_range_loop, manual_strip, approx_constant — flagged in Phase 1), do NOT fix them in this task. The plan deferred them. Note them as a `DONE_WITH_CONCERNS` and continue.

If clippy fails because of NEW lints introduced by removing `dead_code`, address those (they're in scope for this task).

- [ ] **Step 4: Run tests**

```bash
cargo test --workspace 2>&1 | grep -E "^test result:" | awk -F'[ ;]' '{sum+=$4} END {print "Total:", sum}'
```

Expected: `Total: 114`.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
chore(cli): drop crate-level #![allow(dead_code)] from main.rs

After the workspace split, dead-code suppression at the binary's crate
root is no longer load-bearing — the CLI doesn't use any of vaultdb-core's
public API by accident, and any genuine internal-helper dead code that
surfaces is either fixable (delete) or specific (item-level allow with
a reason). Keeping crate-wide allows obscured things; removing it
keeps surfaces honest.

Resolves a Phase 1 deferred item.
EOF
)"
```

---

## Task 9: Final verification

**Goal:** Confirm the phase delivered cleanly. No new commits unless something needs fixing.

- [ ] **Step 1: Full test pass**

```bash
cd /home/rusen/Desktop/codebase-shared/researches/vaultdb/.worktrees/phase2a-foundation-types
cargo test --workspace 2>&1 | grep -E "^test result:" | awk -F'[ ;]' '{sum+=$4} END {print "Total:", sum}'
```

Expected: `Total: 114`.

- [ ] **Step 2: Release build**

```bash
cargo build --release 2>&1 | tail -3
```

Expected: builds cleanly, produces `target/release/vaultdb`.

- [ ] **Step 3: Smoke test against a fresh vault**

```bash
tmpdir=$(mktemp -d) && mkdir -p "$tmpdir/.obsidian" "$tmpdir/notes" && \
    echo -e "---\ntags: [topic/test]\n---\nHello [[other]]" > "$tmpdir/notes/example.md" && \
    ./target/release/vaultdb --vault "$tmpdir" query notes --select "_name,_link_count" && \
    rm -rf "$tmpdir"
```

Expected: prints a table with one row, `example` with link count 1.

- [ ] **Step 4: Confirm the public API surface**

```bash
grep -E "^pub use" crates/vaultdb-core/src/lib.rs
```

Expected output:

```
pub use record::{Record, Value};
pub use error::ParseError;
pub use vault::{LoadResult, Vault};
```

- [ ] **Step 5: Branch history review**

```bash
git log --oneline main..HEAD
```

Expected: 7 commits, one per task (Task 1 was setup-only with no commit; Tasks 2–8 each add one commit; Task 9 adds none unless fixes were needed). Commit messages should match the prescribed forms in each task.

---

## Open questions / followups (out of scope for Phase 2a)

These were considered and deliberately deferred:

- **Public query AST** (`Expr`, `Predicate`, `LinkPredicate`) — Phase 2b.
- **Mutation builders** (`UpdateBuilder` and siblings) — Phase 2b.
- **`LinkGraph` public type** with `GraphScope` — Phase 2b.
- **Migrating CLI commands to use new builders** — Phase 2b.
- **`Serialize`/`Deserialize` on `Record` and `LoadResult`** — only added to `Value` and `ParseError` here. The trickier ones (where `Record::path` is a `PathBuf`, which serializes but produces non-portable strings) get addressed in Phase 2b alongside the wire-format design.
- **The 7 pre-existing clippy lints** flagged in Phase 1 — defer to a later cleanup pass; not blocking.
- **`[workspace.package]` field inheritance** (e.g., `version.workspace = true`) — defer.
- **Module-level rustdoc** for each engine module — defer to Phase 2b's surface freeze.
- **vaultdb-core `From<serde_yaml::Value> for Value`** — add when a real consumer (eduport-core, mcp) needs to construct `Value` from raw YAML. YAGNI for now.
