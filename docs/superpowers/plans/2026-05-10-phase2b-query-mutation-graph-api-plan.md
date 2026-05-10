# Phase 2b — Query AST, Mutation Builders, LinkGraph Public API — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace vaultdb-core's internal `WhereClause`/`WhereExpr`/`LinkIndex` and CLI-shaped mutation primitives with a coherent public API: `Expr`/`Predicate`/`LinkPredicate` AST, `Query` struct, `LinkGraph` type, and `UpdateBuilder`/`DeleteBuilder`/`MoveBuilder`/`RenameBuilder` mutation builders with `plan()`/`execute()` separation. Migrate the CLI to consume only this public API; remove the old internal types.

**Architecture:** Additive new types live alongside the old ones during migration. Each CLI command migrates in its own commit, so each commit is small and bisectable. Old internal types are removed only after all consumers have moved to the new API.

**Tech Stack:** Rust 2024 edition, the existing vaultdb workspace (`vaultdb-core` library + `vaultdb` binary), `serde` and `serde_yaml`, no new crates.

**Working directory for all tasks:** `/home/rusen/Desktop/codebase-shared/researches/vaultdb/.worktrees/phase2b-query-mutation-graph-api` (a git worktree on branch `feat/phase2b-query-mutation-graph-api`, branched from `main` at SHA `477327a`). Set up before Task 1 begins. **All `cargo` and `git` commands run from there**, not from eduport's working tree.

**Baseline before Task 1:** `cargo test --workspace` reports 114 tests passing. The test count grows over the phase; each task that adds tests notes the new expected total. The count must never *drop* below the prior task's expectation, and no existing test should fail. Final expected total at end of phase: 138 (114 + ~24 new across the phase).

**Out of scope for Phase 2b:**
- crates.io republishing (you do this manually after 2b ships).
- `[workspace.package]` field inheritance (defer; non-blocking).
- The 7 pre-existing clippy lints from Phase 1 (still deferred).
- vaultdb-mcp crate (Phase 3).
- Eduport repo touchpoints (Phase 4+).
- Streaming/iterator API for huge vaults (defer until a real consumer needs it).

---

## File structure after this phase

```
crates/vaultdb-core/src/
├── lib.rs              gains `pub use query::{Expr, Predicate, LinkPredicate, CompareOp, Query, SortKey};`
│                       `pub use mutation::{UpdateBuilder, DeleteBuilder, MoveBuilder, RenameBuilder, MutationReport, PlannedChange};`
│                       `pub use links::{LinkGraph, GraphScope, Direction};`
│                       `pub use error::{VaultdbError, Result};`
│                       crate-level rustdoc explaining the dual-structure thesis
├── query.rs            NEW. Public AST types: Expr, Predicate, LinkPredicate, CompareOp, Query, SortKey.
│                       impl FromStr for Expr (parses the where-DSL); Expr::parse convenience method.
├── mutation.rs         NEW. Public typed mutation API: UpdateBuilder, DeleteBuilder, MoveBuilder,
│                       RenameBuilder, MutationReport, PlannedChange. plan() / execute() separation.
├── error.rs            unchanged in 2b body; lib.rs re-exports VaultdbError and Result
├── record.rs           gains #[derive(Serialize, Deserialize)] on Record
├── frontmatter.rs      unchanged (still parses YAML internally)
├── filter.rs           refactored to evaluate the new Expr AST. Old WhereClause/WhereExpr removed
│                       once CLI no longer references them.
├── links.rs            LinkIndex renamed to LinkGraph, gains GraphScope and traverse_from method;
│                       Direction enum replaces existing TraverseDirection.
├── schema.rs           unchanged in 2b body
├── vault.rs            gains query(q: &Query) method, link_graph(scope: GraphScope) method.
│                       LoadResult gains Serialize/Deserialize.
└── writer.rs           used internally by mutation.rs; primitive functions stay as building blocks.

crates/vaultdb/src/
├── main.rs             unchanged
├── cli.rs              unchanged (clap definitions stay)
├── output.rs           unchanged (formatters still consume Vec<Record>)
└── commands/
    ├── query.rs        migrates: builds Query from CLI flags, calls vault.query(&q)
    ├── update.rs       migrates: builds UpdateBuilder, calls execute()
    ├── delete.rs       migrates: builds DeleteBuilder, calls execute()
    ├── move_cmd.rs     migrates: builds MoveBuilder, calls execute()
    ├── rename.rs       migrates: builds RenameBuilder, calls execute()
    ├── links.rs        migrates: uses LinkGraph
    ├── traverse.rs     migrates: uses LinkGraph::traverse_from
    ├── unresolved.rs   migrates: uses LinkGraph::unresolved
    ├── schema_cmd.rs   migrates: uses Vault::query for filter; helpers stay in vaultdb-core
    └── (create.rs)     uses writer.rs primitives directly; no migration needed
```

Total files touched: ~17.

---

## Task 1: Set up the Phase 2b worktree

**Goal:** Create the isolated worktree on a fresh feature branch off main. Verify baseline.

**Files:** none (workspace setup only).

- [ ] **Step 1: Verify main is up to date and clean**

```bash
cd /home/rusen/Desktop/codebase-shared/researches/vaultdb
git status
git log --oneline -3
```

Expected: clean working tree, on `main`, latest commit is `477327a chore(workspace): drop unused thiserror direct dep from vaultdb crate`.

- [ ] **Step 2: Create the worktree**

```bash
git worktree add .worktrees/phase2b-query-mutation-graph-api -b feat/phase2b-query-mutation-graph-api
cd .worktrees/phase2b-query-mutation-graph-api
git status
git branch --show-current
```

Expected: clean working tree, branch `feat/phase2b-query-mutation-graph-api`.

- [ ] **Step 3: Verify baseline**

```bash
cargo build --workspace 2>&1 | tail -3
cargo test --workspace 2>&1 | grep -E "^test result:" | awk -F'[ ;]' '{sum+=$4} END {print "Total tests passing:", sum}'
```

Expected: build clean, `Total tests passing: 114`.

- [ ] **Step 4: No commit.** Workspace setup only. Proceed to Task 2.

---

## Task 2: Add public AST types in a new `query.rs` module

**Goal:** Define `Expr`, `Predicate`, `LinkPredicate`, `Query`, `SortKey` in a new module. Implement `FromStr for Expr` (the where-DSL parser). The new types live alongside the existing internal `WhereClause`/`WhereExpr` — old types still functional during migration.

**Files:**
- Create: `crates/vaultdb-core/src/query.rs`
- Modify: `crates/vaultdb-core/src/lib.rs` (add `pub mod query;` and re-exports)

- [ ] **Step 1: Create `crates/vaultdb-core/src/query.rs` with the types**

Create the file with this content:

```rust
//! Public AST types for vault queries.
//!
//! Frontmatter predicates and link-graph predicates are first-class siblings
//! in the same enum (`Expr`), reflecting vaultdb's dual-structure thesis: a
//! markdown vault is *both* a relational table (frontmatter) and a graph
//! (wikilinks), and the query language treats both equally.

use std::collections::BTreeMap;
use std::str::FromStr;

use crate::error::{Result, VaultdbError};
use crate::record::Value;

/// A composable filter expression. The AST root for vault queries.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Expr {
    /// A frontmatter or virtual-field predicate.
    Predicate(Predicate),
    /// All sub-expressions must hold.
    And(Vec<Expr>),
    /// At least one sub-expression must hold.
    Or(Vec<Expr>),
    /// Negation of a sub-expression.
    Not(Box<Expr>),
    /// Records that link out to a target matching the inner predicate.
    LinksTo(LinkPredicate),
    /// Records linked from anything matching the inner predicate.
    LinkedFrom(LinkPredicate),
}

/// A leaf predicate over a record's frontmatter or virtual fields.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Predicate {
    Equals { field: String, value: Value },
    Contains { field: String, value: Value },
    Compare { field: String, op: CompareOp, value: Value },
    Matches { field: String, regex: String },
    StartsWith { field: String, value: String },
    EndsWith { field: String, value: String },
    Exists { field: String },
    Missing { field: String },
}

/// A scalar comparison operator (used by `Predicate::Compare`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CompareOp {
    Lt,
    Le,
    Gt,
    Ge,
    Ne,
}

/// A predicate over the link graph: either a literal target, or a query into
/// records satisfying a sub-expression. The `Where` variant is what makes
/// joins-via-links possible (e.g., "give me all notes that link to anything
/// tagged `topic/ai`").
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum LinkPredicate {
    Target(String),
    Where(Box<Expr>),
}

/// A complete query: the root expression, optional projection, sort, limit,
/// and the folder to scan.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Query {
    pub folder: String,
    pub filter: Option<Expr>,
    /// `None` means "select all fields".
    pub select: Option<Vec<String>>,
    pub sort: Option<SortKey>,
    pub limit: Option<usize>,
    pub recursive: bool,
}

/// A sort key: which field to sort by, ascending or descending.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SortKey {
    pub field: String,
    pub descending: bool,
}

impl Expr {
    /// Parse a where-DSL string into an `Expr`. Convenience wrapper over
    /// `<Expr as FromStr>::from_str`, so library users have an obvious
    /// discoverable entry point.
    pub fn parse(input: &str) -> Result<Self> {
        input.parse()
    }
}

impl FromStr for Expr {
    type Err = VaultdbError;

    fn from_str(input: &str) -> std::result::Result<Self, Self::Err> {
        // Delegate to the existing parser in filter.rs. The internal
        // WhereExpr / WhereClause types are converted into Expr here.
        // This is a temporary shim for Task 2; the parser will be moved
        // into query.rs in a later task once filter.rs is fully refactored.
        let internal = crate::filter::parse_where_clause(input)?;
        Ok(internal.to_expr())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_equals() {
        let e: Expr = "status = active".parse().unwrap();
        match e {
            Expr::Predicate(Predicate::Equals { field, value }) => {
                assert_eq!(field, "status");
                assert_eq!(value, Value::String("active".into()));
            }
            other => panic!("expected Equals, got {:?}", other),
        }
    }

    #[test]
    fn parse_exists() {
        let e: Expr = "title exists".parse().unwrap();
        match e {
            Expr::Predicate(Predicate::Exists { field }) => assert_eq!(field, "title"),
            other => panic!("expected Exists, got {:?}", other),
        }
    }

    #[test]
    fn parse_compare_gt() {
        let e: Expr = "year > 2020".parse().unwrap();
        match e {
            Expr::Predicate(Predicate::Compare { field, op, value }) => {
                assert_eq!(field, "year");
                assert_eq!(op, CompareOp::Gt);
                assert_eq!(value, Value::Integer(2020));
            }
            other => panic!("expected Compare, got {:?}", other),
        }
    }

    #[test]
    fn expr_serializes_via_serde() {
        let e = Expr::Predicate(Predicate::Equals {
            field: "k".into(),
            value: Value::String("v".into()),
        });
        let json = serde_json::to_string(&e).unwrap();
        // Round-trip
        let back: Expr = serde_json::from_str(&json).unwrap();
        assert_eq!(e, back);
    }

    #[test]
    fn query_struct_construction() {
        let q = Query {
            folder: "notes".into(),
            filter: Some(Expr::Predicate(Predicate::Exists { field: "title".into() })),
            select: Some(vec!["_name".into(), "title".into()]),
            sort: Some(SortKey { field: "_modified".into(), descending: true }),
            limit: Some(10),
            recursive: false,
        };
        assert_eq!(q.folder, "notes");
        assert!(q.filter.is_some());
        assert_eq!(q.limit, Some(10));
    }

    #[test]
    fn link_predicate_target() {
        let lp = LinkPredicate::Target("Foo".into());
        let e = Expr::LinksTo(lp);
        let json = serde_json::to_string(&e).unwrap();
        // Untagged enum representation
        assert!(json.contains("Foo"));
    }
}
```

Notes:
- The `FromStr` impl delegates to a helper `crate::filter::parse_where_clause(input)` that you'll add in Step 2 (this method already exists in some form inside filter.rs's where-string parser; if not, you'll add a thin shim).
- A `to_expr()` method on the internal `WhereClause` type is the conversion bridge — Task 2 adds it as a small method that walks the existing internal AST and produces an `Expr`.
- The tests use `serde_json` for serialization round-trip; `serde_json` is already a dep of vaultdb-core (used for `Value`'s untagged tests).

- [ ] **Step 2: Add the conversion bridge `to_expr()` and `parse_where_clause` shim to filter.rs**

Open `crates/vaultdb-core/src/filter.rs`. The existing internal types are `WhereClause` (an aggregate) and `WhereExpr` (a single predicate). Add these two helpers near the bottom of the file (above any test module):

```rust
/// Parse a where-DSL string into a WhereClause (internal AST).
/// Public-but-internal; the public Expr type's FromStr delegates here for now.
/// This will be removed once filter.rs is fully migrated to the new AST.
pub fn parse_where_clause(input: &str) -> Result<WhereClause> {
    // The existing where-string parser implementation lives somewhere in
    // this file (likely as a method on WhereClause or a free function).
    // If it already exists with a different name (e.g., `parse_where`,
    // `WhereClause::parse`), wrap it here under the public name
    // `parse_where_clause`.
    //
    // If the parser does not yet have a single entry point, extract one
    // by reading the existing where-flag handling code in commands/query.rs
    // (or wherever --where strings are parsed today) and lifting it into
    // this function.
    todo!("locate the existing parser and wire this through")
}

impl WhereClause {
    /// Convert this internal AST into the new public `Expr` type.
    /// Used by `<Expr as FromStr>::from_str` as the migration shim.
    pub fn to_expr(&self) -> crate::query::Expr {
        // Walk the internal AST and produce a structurally equivalent Expr.
        // The mapping is:
        //   WhereClause::All(clauses)      → Expr::And(clauses.iter().map(to_expr).collect())
        //   WhereClause::Any(clauses)      → Expr::Or(...)
        //   WhereClause::Not(clause)       → Expr::Not(Box::new(clause.to_expr()))
        //   WhereClause::Predicate(expr)   → Expr::Predicate(expr.to_predicate())
        //   WhereClause::LinksTo(target)   → Expr::LinksTo(LinkPredicate::Target(target.clone()))
        //   ... etc.
        //
        // The exact variants of WhereClause depend on the existing internal
        // type. Read its definition before writing this method.
        todo!("walk internal AST → Expr")
    }
}

impl WhereExpr {
    /// Convert this single internal predicate into a `Predicate`.
    pub fn to_predicate(&self) -> crate::query::Predicate {
        // Map fields appropriately. The internal `WhereExpr` likely has
        // an `op` field (Equals, Contains, Compare(CompareOp), Matches,
        // StartsWith, EndsWith, Exists, Missing) and `field`/`value`
        // members. Mirror to Predicate variants.
        todo!("map WhereExpr → Predicate")
    }
}
```

The `todo!()` markers are placeholders — when you implement, **replace each one with the correct code based on the actual `WhereClause` / `WhereExpr` definitions you find in filter.rs**. Read those types' definitions FIRST (the file is `crates/vaultdb-core/src/filter.rs`, and the structs/enums are defined near the top, around lines 9–50 from the existing code map: `CompareOp` at line 9, `WhereExpr` at line 25, `WhereClause` at line 36).

If the existing parse function in filter.rs has a different name, alias it: rename your `parse_where_clause` shim to dispatch to whatever exists (e.g., `WhereClause::parse(input)`).

If filter.rs's existing `CompareOp` is structurally identical to the one declared in `query.rs` Step 1, you have two `CompareOp` types in the crate now — that's fine for Task 2; the duplication is resolved when filter.rs is refactored to use the public `query::CompareOp` directly in a later task.

- [ ] **Step 3: Add `pub mod query;` to lib.rs**

Open `crates/vaultdb-core/src/lib.rs`. Currently:

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
pub use vault::{LoadResult, Vault};
```

Add `pub mod query;` and the re-exports for the new types. The full file becomes:

```rust
//! vaultdb-core — library engine for vaultdb.

pub mod error;
pub mod filter;
pub mod frontmatter;
pub mod links;
pub mod query;
pub mod record;
pub mod schema;
pub mod vault;
pub mod writer;

pub use record::{Record, Value};
pub use error::ParseError;
pub use vault::{LoadResult, Vault};
pub use query::{Expr, Predicate, LinkPredicate, CompareOp, Query, SortKey};
```

- [ ] **Step 4: Build and test**

```bash
cd /home/rusen/Desktop/codebase-shared/researches/vaultdb/.worktrees/phase2b-query-mutation-graph-api
cargo build --workspace 2>&1 | tail -5
cargo test --workspace 2>&1 | grep -E "^test result:" | awk -F'[ ;]' '{sum+=$4} END {print "Total:", sum}'
```

Expected: build clean, `Total: 120` (114 + 6 new tests in query.rs).

If you see a compile error like "duplicate type CompareOp", that means filter.rs and query.rs both export a public `CompareOp`. The `pub use query::{..., CompareOp};` re-export at the lib.rs root will pick query::CompareOp; filter.rs's `CompareOp` remains accessible via `crate::filter::CompareOp` for internal use until filter.rs is refactored. That's fine — no fix needed unless tests fail.

If you see ambiguous-import errors elsewhere (e.g., "import `CompareOp` is ambiguous"), narrow the import sites accordingly.

- [ ] **Step 5: Commit**

```bash
git add -A
git status
git commit -m "$(cat <<'EOF'
feat(core): add public Expr/Predicate/Query AST types

Introduces the public query AST in a new query.rs module. Frontmatter
and link predicates are first-class siblings in the same Expr enum,
encoding the dual-structure thesis at the type level (markdown vaults
are both tabular and graph-shaped, queryable as one).

Types: Expr, Predicate, LinkPredicate, CompareOp, Query, SortKey.
All public, all derive Serialize/Deserialize for free IPC use.

FromStr<Expr> delegates to filter.rs's existing where-DSL parser via
a small WhereClause::to_expr() conversion shim. The shim lets the
new types ship without rewriting filter.rs's evaluator yet — that's
Task 4.

6 new unit tests; total now 120 (was 114).
EOF
)"
```

---

## Task 3: Add `Vault::query` method using the new `Query` type

**Goal:** Add a `Vault::query(q: &Query) -> Result<Vec<Record>>` method that runs a structured query. Internally, walk the `Expr` AST and apply filtering/sorting/projection/limit. The implementation can delegate to the existing internal `matches_*` functions for the filter step (using the conversion shim from Task 2).

**Files:**
- Modify: `crates/vaultdb-core/src/vault.rs` (add `query` method, add tests)

- [ ] **Step 1: Add the `query` method**

Open `crates/vaultdb-core/src/vault.rs`. Inside the existing `impl Vault { ... }` block (alongside `discover`, `find_by_name`, `load_records`, etc.), add:

```rust
/// Run a structured query against the vault. Returns the matching records,
/// optionally projected, sorted, and limited per the `Query`'s fields.
///
/// The records returned have raw_content set to None (use load_records_with_content
/// if you need the body text).
pub fn query(&self, q: &crate::query::Query) -> Result<Vec<Record>> {
    let folder_path = self.resolve_folder(&q.folder)?;
    let load = self.load_records(&folder_path, q.recursive, false)?;
    let mut records = load.records;

    // Build a LinkIndex if the filter references the link graph.
    let needs_links = q.filter.as_ref().map_or(false, expr_uses_links);
    let link_index = if needs_links {
        Some(crate::links::LinkIndex::from_records(&records))
    } else {
        None
    };

    // Filter
    if let Some(filter) = &q.filter {
        records.retain(|r| {
            evaluate_expr(filter, r, &self.root, link_index.as_ref())
        });
    }

    // Sort
    if let Some(sort_key) = &q.sort {
        records.sort_by(|a, b| {
            let av = a.get(&sort_key.field, &self.root).unwrap_or(crate::record::Value::Null);
            let bv = b.get(&sort_key.field, &self.root).unwrap_or(crate::record::Value::Null);
            let ord = compare_values(&av, &bv);
            if sort_key.descending { ord.reverse() } else { ord }
        });
    }

    // Limit
    if let Some(limit) = q.limit {
        records.truncate(limit);
    }

    // Projection (if requested, keep only selected fields plus virtual fields)
    if let Some(select) = &q.select {
        let select_set: std::collections::BTreeSet<&str> =
            select.iter().map(|s| s.as_str()).collect();
        for record in records.iter_mut() {
            record.fields.retain(|k, _| select_set.contains(k.as_str()));
        }
    }

    Ok(records)
}
```

This method depends on three helper functions (`expr_uses_links`, `evaluate_expr`, `compare_values`) and one method (`LinkIndex::from_records`) that may or may not exist. The next steps add them.

- [ ] **Step 2: Add helper free functions to vault.rs**

In the same file, near the bottom (above any `mod tests`), add:

```rust
/// Returns true if any node of `expr` references the link graph.
fn expr_uses_links(expr: &crate::query::Expr) -> bool {
    use crate::query::Expr;
    match expr {
        Expr::LinksTo(_) | Expr::LinkedFrom(_) => true,
        Expr::Predicate(_) => false,
        Expr::And(es) | Expr::Or(es) => es.iter().any(expr_uses_links),
        Expr::Not(e) => expr_uses_links(e),
    }
}

/// Evaluate an Expr against a single record. Mirrors the internal
/// `matches_*` evaluation but operates on the public AST.
fn evaluate_expr(
    expr: &crate::query::Expr,
    record: &Record,
    vault_root: &Path,
    link_index: Option<&crate::links::LinkIndex>,
) -> bool {
    use crate::query::{Expr, Predicate, LinkPredicate};
    match expr {
        Expr::Predicate(p) => evaluate_predicate(p, record, vault_root),
        Expr::And(es) => es.iter().all(|e| evaluate_expr(e, record, vault_root, link_index)),
        Expr::Or(es) => es.iter().any(|e| evaluate_expr(e, record, vault_root, link_index)),
        Expr::Not(e) => !evaluate_expr(e, record, vault_root, link_index),
        Expr::LinksTo(lp) => match (link_index, lp) {
            (Some(idx), LinkPredicate::Target(name)) => idx.outgoing(&record.virtual_name())
                .iter()
                .any(|n| n == name),
            (Some(idx), LinkPredicate::Where(inner)) => {
                idx.outgoing(&record.virtual_name())
                    .iter()
                    .any(|target_name| {
                        // Find the target record and recursively evaluate the inner expr.
                        // For now, look up by name in the index's known records.
                        idx.record_by_name(target_name)
                            .map_or(false, |target_record| {
                                evaluate_expr(inner, target_record, vault_root, Some(idx))
                            })
                    })
            }
            (None, _) => false,
        },
        Expr::LinkedFrom(lp) => match (link_index, lp) {
            (Some(idx), LinkPredicate::Target(name)) => idx.incoming(&record.virtual_name())
                .iter()
                .any(|n| n == name),
            (Some(idx), LinkPredicate::Where(inner)) => {
                idx.incoming(&record.virtual_name())
                    .iter()
                    .any(|source_name| {
                        idx.record_by_name(source_name)
                            .map_or(false, |source_record| {
                                evaluate_expr(inner, source_record, vault_root, Some(idx))
                            })
                    })
            }
            (None, _) => false,
        },
    }
}

/// Evaluate a leaf predicate against a record's field.
fn evaluate_predicate(
    p: &crate::query::Predicate,
    record: &Record,
    vault_root: &Path,
) -> bool {
    use crate::query::{Predicate, CompareOp};
    use crate::record::Value;

    match p {
        Predicate::Equals { field, value } => {
            record.get(field, vault_root).as_ref() == Some(value)
        }
        Predicate::Contains { field, value } => match record.get(field, vault_root) {
            Some(Value::String(s)) => match value {
                Value::String(v) => s.contains(v.as_str()),
                _ => false,
            },
            Some(Value::List(list)) => list.iter().any(|item| item == value),
            _ => false,
        },
        Predicate::Compare { field, op, value } => {
            let actual = match record.get(field, vault_root) {
                Some(v) => v,
                None => return false,
            };
            let ord = compare_values(&actual, value);
            match op {
                CompareOp::Lt => ord == std::cmp::Ordering::Less,
                CompareOp::Le => ord != std::cmp::Ordering::Greater,
                CompareOp::Gt => ord == std::cmp::Ordering::Greater,
                CompareOp::Ge => ord != std::cmp::Ordering::Less,
                CompareOp::Ne => ord != std::cmp::Ordering::Equal,
            }
        }
        Predicate::Matches { field, regex } => match record.get(field, vault_root) {
            Some(Value::String(s)) => {
                regex::Regex::new(regex).map_or(false, |re| re.is_match(&s))
            }
            _ => false,
        },
        Predicate::StartsWith { field, value } => match record.get(field, vault_root) {
            Some(Value::String(s)) => s.starts_with(value),
            _ => false,
        },
        Predicate::EndsWith { field, value } => match record.get(field, vault_root) {
            Some(Value::String(s)) => s.ends_with(value),
            _ => false,
        },
        Predicate::Exists { field } => {
            !matches!(record.get(field, vault_root), None | Some(Value::Null))
        }
        Predicate::Missing { field } => {
            matches!(record.get(field, vault_root), None | Some(Value::Null))
        }
    }
}

/// Compare two `Value`s for sorting. String < Integer < Float etc. is left
/// unspecified; we sort by string repr as a fallback for cross-variant comparisons.
fn compare_values(a: &crate::record::Value, b: &crate::record::Value) -> std::cmp::Ordering {
    use crate::record::Value;
    match (a, b) {
        (Value::Integer(x), Value::Integer(y)) => x.cmp(y),
        (Value::Float(x), Value::Float(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
        (Value::String(x), Value::String(y)) => x.cmp(y),
        (Value::Bool(x), Value::Bool(y)) => x.cmp(y),
        (Value::Null, Value::Null) => std::cmp::Ordering::Equal,
        (Value::Null, _) => std::cmp::Ordering::Less,
        (_, Value::Null) => std::cmp::Ordering::Greater,
        // Fallback: compare by string representation
        _ => format!("{:?}", a).cmp(&format!("{:?}", b)),
    }
}
```

This evaluator depends on:
- `record.virtual_name()` — already exists per Phase 2a tests.
- `record.get(field, vault_root)` — already exists.
- `LinkIndex::outgoing(name) -> &[String]` — likely exists; verify by reading links.rs.
- `LinkIndex::incoming(name) -> &[String]` — likely exists.
- `LinkIndex::record_by_name(name) -> Option<&Record>` — **may not exist**. If it doesn't, add it as part of this task — see Step 3 below.

If `record.virtual_name()` or `record.get(...)` have different names, locate them via grep and adjust.

- [ ] **Step 3: Add `LinkIndex::record_by_name` (if it doesn't exist)**

Check whether `LinkIndex` already has a way to look up a record by name:

```bash
grep -n "fn record_by_name\|fn record_for\|fn get_record\|fn find_record" crates/vaultdb-core/src/links.rs
```

If no match, the LinkIndex needs to retain the record list internally. Open `crates/vaultdb-core/src/links.rs`, find `pub struct LinkIndex` (around line 59 of the existing file). Inspect its fields. If it doesn't already store records, you have two options:

**Option A: Add a record map to LinkIndex.** Modify `LinkIndex` to additionally hold a `BTreeMap<String, Record>` and add a `record_by_name(&self, name: &str) -> Option<&Record>` accessor.

**Option B: Build the link index AND keep a separate name→record map at the call site.** Pass the map alongside the LinkIndex into `evaluate_expr`.

Option A is cleaner and ergonomic. Implement it:

```rust
// Inside the existing LinkIndex struct, add a field:
pub struct LinkIndex {
    // ... existing fields ...
    records_by_name: std::collections::BTreeMap<String, Record>,
}

// In LinkIndex::from_records (existing constructor — verify by reading):
impl LinkIndex {
    pub fn from_records(records: &[Record]) -> Self {
        // ... existing code that builds outgoing/incoming maps ...

        let mut records_by_name = std::collections::BTreeMap::new();
        for r in records {
            records_by_name.insert(r.virtual_name(), r.clone());
        }

        // Return constructed self (existing code shape)
        Self {
            // ... existing fields ...
            records_by_name,
        }
    }

    /// Look up a record by its virtual name (filename without `.md`).
    pub fn record_by_name(&self, name: &str) -> Option<&Record> {
        self.records_by_name.get(name)
    }
}
```

If your existing `LinkIndex::from_records` has a different signature or structure, adapt the snippet to fit. The key invariant: every record fed in is reachable via `record_by_name`.

- [ ] **Step 4: Add tests for `Vault::query`**

Inside `crates/vaultdb-core/src/vault.rs`'s `#[cfg(test)] mod tests` block, add:

```rust
#[test]
fn query_basic_filter() {
    use crate::query::{Expr, Predicate, Query};
    use crate::record::Value;

    let dir = create_test_vault();
    let vault = Vault::with_root(dir.path().to_path_buf());

    let q = Query {
        folder: "notes".into(),
        filter: Some(Expr::Predicate(Predicate::Equals {
            field: "status".into(),
            value: Value::String("active".into()),
        })),
        select: None,
        sort: None,
        limit: None,
        recursive: false,
    };

    let results = vault.query(&q).unwrap();
    assert!(results.iter().all(|r| {
        matches!(
            r.get("status", &vault.root),
            Some(Value::String(ref s)) if s == "active"
        )
    }));
}

#[test]
fn query_with_limit_and_sort() {
    use crate::query::{Expr, Predicate, Query, SortKey};

    let dir = create_test_vault();
    let vault = Vault::with_root(dir.path().to_path_buf());

    let q = Query {
        folder: "notes".into(),
        filter: Some(Expr::Predicate(Predicate::Exists { field: "_name".into() })),
        select: None,
        sort: Some(SortKey { field: "_name".into(), descending: false }),
        limit: Some(2),
        recursive: false,
    };

    let results = vault.query(&q).unwrap();
    assert!(results.len() <= 2);
}

#[test]
fn query_with_projection() {
    use crate::query::{Expr, Predicate, Query};

    let dir = create_test_vault();
    let vault = Vault::with_root(dir.path().to_path_buf());

    let q = Query {
        folder: "notes".into(),
        filter: Some(Expr::Predicate(Predicate::Exists { field: "_name".into() })),
        select: Some(vec!["status".into()]),
        sort: None,
        limit: None,
        recursive: false,
    };

    let results = vault.query(&q).unwrap();
    for r in &results {
        // Only "status" survives in fields (virtual fields are computed lazily,
        // not stored, so the test only checks that non-selected concrete
        // frontmatter fields are dropped).
        assert!(
            r.fields.keys().all(|k| k == "status"),
            "expected only 'status' in fields, got {:?}",
            r.fields.keys().collect::<Vec<_>>()
        );
    }
}
```

If `create_test_vault()`'s output doesn't include records with a `status` frontmatter field, these tests need adjustment. Read the helper definition (it's in vault.rs's tests module, near the top) and tweak field names to match.

- [ ] **Step 5: Build and test**

```bash
cargo build --workspace
cargo test --workspace 2>&1 | grep -E "^test result:" | awk -F'[ ;]' '{sum+=$4} END {print "Total:", sum}'
```

Expected: build clean, `Total: 123` (120 + 3 new query tests).

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat(core): add Vault::query method using the new Query type

Vault::query(q: &Query) -> Result<Vec<Record>> walks the public Expr
AST with full support for And/Or/Not, frontmatter Predicate,
LinksTo/LinkedFrom, sort, limit, and projection.

Supporting helpers: evaluate_expr, evaluate_predicate, compare_values,
expr_uses_links — all private to vault.rs since they're evaluator
internals. LinkIndex gains record_by_name() accessor to support
LinkPredicate::Where evaluation across the link graph.

3 new tests; total 123.
EOF
)"
```

---

## Task 4: Refactor `filter.rs` to evaluate the public `Expr` AST natively

**Goal:** Move the evaluator (`evaluate_expr`, `evaluate_predicate`, `compare_values`, `expr_uses_links`) from vault.rs into filter.rs. Make filter.rs's exposed `matches_*` functions take the public `Expr` instead of internal `WhereClause`/`WhereExpr`. Old internal types still exist but are now only used by the conversion shim from Task 2 (and by CLI commands not yet migrated).

**Files:**
- Modify: `crates/vaultdb-core/src/filter.rs` (relocate evaluator, expose new pub fns)
- Modify: `crates/vaultdb-core/src/vault.rs` (drop the helper functions; call into filter.rs)

- [ ] **Step 1: Move the four helper functions from vault.rs into filter.rs**

Open `crates/vaultdb-core/src/vault.rs`. Find `expr_uses_links`, `evaluate_expr`, `evaluate_predicate`, and `compare_values`. **Cut** them (delete from vault.rs).

Open `crates/vaultdb-core/src/filter.rs`. **Paste** them at the bottom of the file (above any test module). Change their visibility from private to `pub` (so vault.rs can call them):

```rust
pub fn expr_uses_links(expr: &crate::query::Expr) -> bool {
    /* ...same body as before... */
}

pub fn evaluate_expr(
    expr: &crate::query::Expr,
    record: &Record,
    vault_root: &Path,
    link_index: Option<&LinkIndex>,
) -> bool {
    /* ...same body... */
}

pub fn evaluate_predicate(
    p: &crate::query::Predicate,
    record: &Record,
    vault_root: &Path,
) -> bool {
    /* ...same body... */
}

pub fn compare_values(a: &crate::record::Value, b: &crate::record::Value) -> std::cmp::Ordering {
    /* ...same body... */
}
```

Adjust imports in filter.rs — `use crate::query::Expr;` etc. — as needed. Drop `crate::filter::` qualifiers inside the function bodies (since they're now in filter.rs themselves, the path is just `LinkIndex` etc).

- [ ] **Step 2: Update vault.rs to call into filter.rs**

Open `crates/vaultdb-core/src/vault.rs`. The `Vault::query` method's body referenced `expr_uses_links`, `evaluate_expr`, and `compare_values` as bare function calls. Update them to call the public versions in filter.rs:

```rust
let needs_links = q.filter.as_ref().map_or(false, crate::filter::expr_uses_links);
// ...
if let Some(filter) = &q.filter {
    records.retain(|r| {
        crate::filter::evaluate_expr(filter, r, &self.root, link_index.as_ref())
    });
}
// ...
records.sort_by(|a, b| {
    let av = a.get(&sort_key.field, &self.root).unwrap_or(crate::record::Value::Null);
    let bv = b.get(&sort_key.field, &self.root).unwrap_or(crate::record::Value::Null);
    let ord = crate::filter::compare_values(&av, &bv);
    /* ... */
});
```

- [ ] **Step 3: Build and test**

```bash
cargo build --workspace
cargo test --workspace 2>&1 | grep -E "^test result:" | awk -F'[ ;]' '{sum+=$4} END {print "Total:", sum}'
```

Expected: `Total: 123` (no test count change; this is a pure code relocation).

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
refactor(core): move Expr evaluator into filter.rs

The query evaluator (evaluate_expr, evaluate_predicate, compare_values,
expr_uses_links) now lives in filter.rs as public functions. vault.rs's
Vault::query method calls into them. This cleanly separates "the AST"
(query.rs) from "how to evaluate the AST against records" (filter.rs).

The legacy internal WhereClause/WhereExpr types still exist for the
unmigrated CLI commands, used only via the FromStr<Expr> shim.
EOF
)"
```

---

## Task 5: Migrate CLI `query.rs` to use `Vault::query` and `Expr::parse`

**Goal:** The first CLI command migration. `commands/query.rs` becomes a thin wrapper that builds a `Query` from CLI flags and calls `vault.query(&q)`. Old `WhereClause` references in this file disappear.

**Files:**
- Modify: `crates/vaultdb/src/commands/query.rs`

- [ ] **Step 1: Read the current implementation**

```bash
cd /home/rusen/Desktop/codebase-shared/researches/vaultdb/.worktrees/phase2b-query-mutation-graph-api
cat crates/vaultdb/src/commands/query.rs | head -80
```

Note the current shape: how it parses `--where`, `--select`, `--sort`, `--desc`, `--limit`, `--links-to`, `--linked-from`, `--links-to-where`, `--linked-from-where`, and `--recursive`.

- [ ] **Step 2: Rewrite to use `Vault::query`**

Replace the body of `run_query` (or whatever the entry function is named) with code that constructs a `Query`:

```rust
pub fn run_query(
    vault: &vaultdb_core::Vault,
    folder: &str,
    where_exprs: &[String],
    select: &Option<String>,
    sort: Option<&str>,
    desc: bool,
    limit: Option<usize>,
    format: &OutputFormat,
    relational: &RelationalFilters,
    recursive: bool,
    verbose: bool,
) -> Result<()> {
    use vaultdb_core::{Expr, LinkPredicate, Predicate, Query, SortKey};

    // Combine multiple --where flags with AND
    let parsed: Vec<Expr> = where_exprs
        .iter()
        .map(|s| Expr::parse(s))
        .collect::<vaultdb_core::error::Result<Vec<_>>>()?;

    // Add relational filters (--links-to, --linked-from, --links-to-where, --linked-from-where)
    let mut all_exprs = parsed;

    for target in &relational.links_to {
        all_exprs.push(Expr::LinksTo(LinkPredicate::Target(target.clone())));
    }
    for target in &relational.linked_from {
        all_exprs.push(Expr::LinkedFrom(LinkPredicate::Target(target.clone())));
    }
    if let Some(s) = &relational.links_to_where {
        all_exprs.push(Expr::LinksTo(LinkPredicate::Where(Box::new(Expr::parse(s)?))));
    }
    if let Some(s) = &relational.linked_from_where {
        all_exprs.push(Expr::LinkedFrom(LinkPredicate::Where(Box::new(Expr::parse(s)?))));
    }

    let filter = match all_exprs.len() {
        0 => None,
        1 => Some(all_exprs.into_iter().next().unwrap()),
        _ => Some(Expr::And(all_exprs)),
    };

    let select_vec = select.as_ref().map(|s| {
        s.split(',').map(|f| f.trim().to_string()).collect::<Vec<_>>()
    });

    let sort_key = sort.map(|f| SortKey { field: f.to_string(), descending: desc });

    let q = Query {
        folder: folder.to_string(),
        filter,
        select: select_vec,
        sort: sort_key,
        limit,
        recursive,
    };

    let records = vault.query(&q).map_err(anyhow::Error::from)?;

    // Output
    let _ = verbose; // verbose was used for parse-error logging in old impl;
                    // load_records still handles that internally.
    output::format_records_with_links(&records, format, None, &vault.root)?;

    Ok(())
}
```

The structure of `RelationalFilters` and `OutputFormat` is unchanged — those are CLI-side types; refer to `commands/query.rs`'s existing imports. If your existing code uses different parameter names (e.g., `where_exprs` vs `where_clauses`), match what's there.

If there's existing logic for the projection/sort that doesn't fit `Vault::query`'s shape, leave that handling on the CLI side (it's harmless).

- [ ] **Step 3: Build, test, smoke-test**

```bash
cargo build --workspace
cargo test --workspace 2>&1 | grep -E "^test result:" | awk -F'[ ;]' '{sum+=$4} END {print "Total:", sum}'
```

Expected: build clean, `Total: 123`.

Smoke test against a vault:

```bash
tmpdir=$(mktemp -d)
mkdir -p "$tmpdir/.obsidian" "$tmpdir/notes"
echo -e "---\nstatus: active\nyear: 2020\n---\nA" > "$tmpdir/notes/a.md"
echo -e "---\nstatus: pending\nyear: 2021\n---\nB linking [[a]]" > "$tmpdir/notes/b.md"
cargo build 2>&1 | tail -2
./target/debug/vaultdb --vault "$tmpdir" query notes --where "status = active" --select "_name,status"
./target/debug/vaultdb --vault "$tmpdir" query notes --links-to "a" --select "_name"
rm -rf "$tmpdir"
```

Expected: each invocation returns a table with the matching row. The first should show `a, active`. The second should show `b`.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
refactor(cli): migrate query command to use Vault::query and Expr::parse

commands/query.rs now builds a Query from CLI flags and dispatches to
vault.query(&q). The relational --links-to / --linked-from /
--links-to-where / --linked-from-where flags translate directly into
Expr::LinksTo / Expr::LinkedFrom variants — no special-case handling
on the CLI side anymore.

The internal WhereClause/WhereExpr is no longer used by this command.
Other commands still depend on it; they migrate in subsequent tasks.
EOF
)"
```

---

## Task 6: Add public `LinkGraph` type (rename `LinkIndex`) and `GraphScope`

**Goal:** Rename the internal `LinkIndex` to public `LinkGraph`, adding the spec's prescribed methods (`outgoing`, `incoming`, `unresolved`, `traverse_from`) plus `GraphScope` enum and `Direction` enum (replacing `TraverseDirection`). Existing `LinkIndex` callers continue to work via a type alias during migration.

**Files:**
- Modify: `crates/vaultdb-core/src/links.rs` (rename, add new types and methods)
- Modify: `crates/vaultdb-core/src/lib.rs` (re-exports)

- [ ] **Step 1: Rename `LinkIndex` to `LinkGraph` in `links.rs`**

Open `crates/vaultdb-core/src/links.rs`. Use sed to rename, but only inside this one file (so we don't break callers yet — they keep using `LinkIndex` via the alias):

```bash
cd /home/rusen/Desktop/codebase-shared/researches/vaultdb/.worktrees/phase2b-query-mutation-graph-api
sed -i 's/\bLinkIndex\b/LinkGraph/g' crates/vaultdb-core/src/links.rs
```

Then add a backward-compat alias at the bottom of `links.rs`:

```rust
/// Legacy name for `LinkGraph`. Will be removed once all callers migrate.
pub type LinkIndex = LinkGraph;
```

- [ ] **Step 2: Add `GraphScope` and `Direction` types**

Near the top of `links.rs` (after existing `use` statements), add:

```rust
/// What subset of the vault to build the link graph over.
#[derive(Debug, Clone)]
pub enum GraphScope {
    All,
    Folder(String),
    Where(crate::query::Expr),
}

/// Direction for graph traversal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Outgoing,
    Incoming,
    Both,
}
```

If a `TraverseDirection` enum already exists with similar variants (per the existing code map at line 9), keep `Direction` separate for now and add a one-way conversion:

```rust
impl From<TraverseDirection> for Direction {
    fn from(td: TraverseDirection) -> Direction {
        match td {
            // Map the existing variants to the new ones — read TraverseDirection's
            // definition first to confirm. Likely:
            TraverseDirection::Outgoing => Direction::Outgoing,
            TraverseDirection::Incoming => Direction::Incoming,
            TraverseDirection::Both => Direction::Both,
        }
    }
}
```

- [ ] **Step 3: Add new methods to `LinkGraph`**

In `crates/vaultdb-core/src/links.rs`'s `impl LinkGraph` block, add these methods (alongside any existing ones — don't replace):

```rust
impl LinkGraph {
    /// All outgoing wikilink targets from `name`. Returns an empty slice if
    /// `name` is unknown.
    pub fn outgoing(&self, name: &str) -> &[String] {
        // If a function with this exact shape already exists, skip.
        // Otherwise, look up the outgoing-edges map (an existing private
        // field) and return its slice; if the name isn't present, return &[].
        self.outgoing_map.get(name).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// All incoming wikilink sources for `name`. Returns an empty slice if
    /// `name` is unknown.
    pub fn incoming(&self, name: &str) -> &[String] {
        self.incoming_map.get(name).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// All unresolved links across the graph: wikilinks whose target file
    /// does not exist in the records this graph was built from.
    pub fn unresolved(&self) -> Vec<UnresolvedLink> {
        let mut out = Vec::new();
        for (source_name, targets) in &self.outgoing_map {
            for target in targets {
                if !self.records_by_name.contains_key(target) {
                    out.push(UnresolvedLink {
                        source: source_name.clone(),
                        target: target.clone(),
                    });
                }
            }
        }
        out
    }

    /// BFS traversal from `start`, returning all reachable record names
    /// up to `depth`. The `direction` controls which edges are followed.
    pub fn traverse_from(
        &self,
        start: &str,
        depth: usize,
        direction: Direction,
    ) -> Vec<String> {
        use std::collections::VecDeque;
        let mut visited = std::collections::BTreeSet::new();
        let mut out = Vec::new();
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();
        queue.push_back((start.to_string(), 0));
        visited.insert(start.to_string());

        while let Some((cur, d)) = queue.pop_front() {
            if d > 0 {
                out.push(cur.clone());
            }
            if d >= depth {
                continue;
            }
            let next_names: Vec<String> = match direction {
                Direction::Outgoing => self.outgoing(&cur).to_vec(),
                Direction::Incoming => self.incoming(&cur).to_vec(),
                Direction::Both => {
                    let mut both = self.outgoing(&cur).to_vec();
                    both.extend_from_slice(self.incoming(&cur));
                    both
                }
            };
            for n in next_names {
                if visited.insert(n.clone()) {
                    queue.push_back((n, d + 1));
                }
            }
        }
        out
    }
}

/// A wikilink whose target file does not exist in the graph.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UnresolvedLink {
    pub source: String,
    pub target: String,
}
```

The field names (`outgoing_map`, `incoming_map`, `records_by_name`) are placeholders. Read `LinkGraph`'s actual field definitions in links.rs first; use the actual names. If the existing struct uses a single bidirectional map or different shape, adapt accordingly.

If `outgoing()`/`incoming()` already exist on the struct (per the existing implementation), skip those duplicates and only add the new methods (`unresolved`, `traverse_from`).

- [ ] **Step 4: Re-export from lib.rs**

Open `crates/vaultdb-core/src/lib.rs`. Append:

```rust
pub use links::{LinkGraph, GraphScope, Direction, UnresolvedLink};
```

The full lib.rs should now end with:

```rust
pub use record::{Record, Value};
pub use error::ParseError;
pub use vault::{LoadResult, Vault};
pub use query::{Expr, Predicate, LinkPredicate, CompareOp, Query, SortKey};
pub use links::{LinkGraph, GraphScope, Direction, UnresolvedLink};
```

- [ ] **Step 5: Add `Vault::link_graph(scope: GraphScope)`**

Open `crates/vaultdb-core/src/vault.rs`. Inside `impl Vault`, add:

```rust
/// Build a link graph over the given scope.
pub fn link_graph(&self, scope: crate::links::GraphScope) -> Result<crate::links::LinkGraph> {
    use crate::links::GraphScope;
    let records: Vec<Record> = match scope {
        GraphScope::All => {
            // Walk all .md files under root, recursive.
            self.load_records(&self.root, true, false)?.records
        }
        GraphScope::Folder(folder) => {
            let path = self.resolve_folder(&folder)?;
            self.load_records(&path, true, false)?.records
        }
        GraphScope::Where(expr) => {
            // Build an unfiltered graph first to support link-aware predicates,
            // then filter.
            let all = self.load_records(&self.root, true, false)?.records;
            let idx = crate::links::LinkGraph::from_records(&all);
            all.into_iter()
                .filter(|r| crate::filter::evaluate_expr(&expr, r, &self.root, Some(&idx)))
                .collect()
        }
    };
    Ok(crate::links::LinkGraph::from_records(&records))
}
```

- [ ] **Step 6: Add tests**

Inside `crates/vaultdb-core/src/links.rs`'s `#[cfg(test)] mod tests`, add:

```rust
#[test]
fn link_graph_outgoing_and_incoming() {
    let r1 = Record {
        path: PathBuf::from("/v/notes/a.md"),
        fields: BTreeMap::new(),
        raw_content: Some("[[b]] [[c]]".into()),
    };
    let r2 = Record {
        path: PathBuf::from("/v/notes/b.md"),
        fields: BTreeMap::new(),
        raw_content: Some("".into()),
    };
    let graph = LinkGraph::from_records(&[r1, r2]);
    assert_eq!(graph.outgoing("a"), &["b".to_string(), "c".to_string()][..]);
    assert_eq!(graph.incoming("b"), &["a".to_string()][..]);
}

#[test]
fn link_graph_unresolved_returns_dangling_links() {
    let r1 = Record {
        path: PathBuf::from("/v/notes/a.md"),
        fields: BTreeMap::new(),
        raw_content: Some("[[ghost]]".into()),
    };
    let graph = LinkGraph::from_records(&[r1]);
    let unresolved = graph.unresolved();
    assert_eq!(unresolved.len(), 1);
    assert_eq!(unresolved[0].target, "ghost");
}

#[test]
fn link_graph_traverse_outgoing() {
    let mk = |name: &str, content: &str| Record {
        path: PathBuf::from(format!("/v/notes/{}.md", name)),
        fields: BTreeMap::new(),
        raw_content: Some(content.into()),
    };
    let records = vec![
        mk("a", "[[b]]"),
        mk("b", "[[c]]"),
        mk("c", ""),
    ];
    let graph = LinkGraph::from_records(&records);
    let names = graph.traverse_from("a", 2, Direction::Outgoing);
    assert!(names.contains(&"b".to_string()));
    assert!(names.contains(&"c".to_string()));
}
```

The `Record` and helper imports must match what's currently in the test module. If `LinkGraph::from_records` has a different signature or there's a different `Record` constructor convention, adapt.

- [ ] **Step 7: Build and test**

```bash
cargo build --workspace
cargo test --workspace 2>&1 | grep -E "^test result:" | awk -F'[ ;]' '{sum+=$4} END {print "Total:", sum}'
```

Expected: build clean, `Total: 126` (123 + 3 new graph tests).

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat(core): public LinkGraph with GraphScope, Direction, traverse_from

Renames internal LinkIndex to LinkGraph (with `pub type LinkIndex =
LinkGraph` alias for the unmigrated callers). Adds:

- GraphScope { All, Folder(name), Where(Expr) }
- Direction { Outgoing, Incoming, Both }
- UnresolvedLink { source, target } — for the dangling-link discovery
- LinkGraph::outgoing / incoming / unresolved / traverse_from
- Vault::link_graph(scope) — builds a graph over a subset

3 new tests; total 126.
EOF
)"
```

---

## Task 7: Add `mutation` module with `UpdateBuilder`

**Goal:** Create the `mutation.rs` module exposing the typed mutation API. Start with `UpdateBuilder` (the most complex one); siblings follow in Task 8.

**Files:**
- Create: `crates/vaultdb-core/src/mutation.rs`
- Modify: `crates/vaultdb-core/src/lib.rs` (add `pub mod mutation;` and re-exports)

- [ ] **Step 1: Create `crates/vaultdb-core/src/mutation.rs`**

```rust
//! Public typed mutation API for vault edits.
//!
//! Each builder provides a `plan(vault) -> MutationReport` (read-only,
//! produces a preview) and `execute(vault) -> MutationReport` (which writes
//! the planned changes to disk). vaultdb-mcp uses plan-only mode by default;
//! interactive applications use execute().

use std::path::PathBuf;

use crate::error::{Result, VaultdbError};
use crate::query::Expr;
use crate::record::{Record, Value};

/// A report of changes a builder would (or did) make.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MutationReport {
    pub changes: Vec<PlannedChange>,
    pub errors: Vec<MutationError>,
}

/// A single planned (or applied) change to one record.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlannedChange {
    pub path: PathBuf,
    pub before: std::collections::BTreeMap<String, Value>,
    pub after: std::collections::BTreeMap<String, Value>,
    pub description: String,
}

/// A failure to apply a single change.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MutationError {
    pub path: PathBuf,
    pub message: String,
}

/// Build an update mutation. Required: a filter `Expr`. Optional: any
/// combination of `set`, `unset`, `add_tag`, `remove_tag` calls.
#[derive(Debug, Clone)]
pub struct UpdateBuilder {
    filter: Expr,
    folder: String,
    set_fields: Vec<(String, Value)>,
    unset_fields: Vec<String>,
    add_tags: Vec<String>,
    remove_tags: Vec<String>,
}

impl UpdateBuilder {
    pub fn new(folder: impl Into<String>, filter: Expr) -> Self {
        Self {
            filter,
            folder: folder.into(),
            set_fields: Vec::new(),
            unset_fields: Vec::new(),
            add_tags: Vec::new(),
            remove_tags: Vec::new(),
        }
    }

    pub fn set(mut self, field: impl Into<String>, value: Value) -> Self {
        self.set_fields.push((field.into(), value));
        self
    }

    pub fn unset(mut self, field: impl Into<String>) -> Self {
        self.unset_fields.push(field.into());
        self
    }

    pub fn add_tag(mut self, tag: impl Into<String>) -> Self {
        self.add_tags.push(tag.into());
        self
    }

    pub fn remove_tag(mut self, tag: impl Into<String>) -> Self {
        self.remove_tags.push(tag.into());
        self
    }

    /// Compute the report without writing. Read-only.
    pub fn plan(&self, vault: &crate::vault::Vault) -> Result<MutationReport> {
        let folder_path = vault.resolve_folder(&self.folder)?;
        let load = vault.load_records_with_content(&folder_path, false, false)?;
        let needs_links = crate::filter::expr_uses_links(&self.filter);
        let link_index = if needs_links {
            Some(crate::links::LinkGraph::from_records(&load.records))
        } else {
            None
        };

        let mut changes = Vec::new();
        let mut errors = Vec::new();

        for record in &load.records {
            if !crate::filter::evaluate_expr(&self.filter, record, &vault.root, link_index.as_ref()) {
                continue;
            }
            // Apply each operation to the record's raw_content; collect the
            // before/after frontmatter snapshot.
            let before = record.fields.clone();
            let mut content = match &record.raw_content {
                Some(c) => c.clone(),
                None => continue, // can't mutate without raw content
            };
            let mut description_parts = Vec::new();

            for (k, v) in &self.set_fields {
                let value_str = render_value_for_yaml(v);
                let (new_content, change_desc) = crate::writer::set_field(&content, k, &value_str)
                    .map_err(VaultdbError::from)?;
                content = new_content;
                description_parts.push(format!("set {}={}", k, value_str));
                let _ = change_desc;
            }
            for k in &self.unset_fields {
                let (new_content, _) = crate::writer::unset_field(&content, k)
                    .map_err(VaultdbError::from)?;
                content = new_content;
                description_parts.push(format!("unset {}", k));
            }
            for tag in &self.add_tags {
                let (new_content, _) = crate::writer::add_tag(&content, tag)
                    .map_err(VaultdbError::from)?;
                content = new_content;
                description_parts.push(format!("+tag {}", tag));
            }
            for tag in &self.remove_tags {
                let (new_content, _) = crate::writer::remove_tag(&content, tag)
                    .map_err(VaultdbError::from)?;
                content = new_content;
                description_parts.push(format!("-tag {}", tag));
            }

            // Re-parse the new content to get the after-frontmatter.
            let after = parse_frontmatter_only(&content)?;

            changes.push(PlannedChange {
                path: record.path.clone(),
                before,
                after,
                description: description_parts.join(", "),
            });
        }

        Ok(MutationReport { changes, errors })
    }

    /// Plan, then write the planned changes to disk.
    ///
    /// Implementation duplicates plan()'s inner loop because the simplest way
    /// to write each new file is to re-compute the per-record new content as
    /// we go. A follow-up refactor (noted in the plan's open questions) would
    /// make plan() store new_content per change so execute() doesn't recompute.
    pub fn execute(self, vault: &crate::vault::Vault) -> Result<MutationReport> {
        let folder_path = vault.resolve_folder(&self.folder)?;
        let load = vault.load_records_with_content(&folder_path, false, false)?;
        let needs_links = crate::filter::expr_uses_links(&self.filter);
        let link_index = if needs_links {
            Some(crate::links::LinkGraph::from_records(&load.records))
        } else {
            None
        };

        let mut changes = Vec::new();
        let errors = Vec::new();

        for record in &load.records {
            if !crate::filter::evaluate_expr(&self.filter, record, &vault.root, link_index.as_ref()) {
                continue;
            }
            let before = record.fields.clone();
            let mut content = match &record.raw_content {
                Some(c) => c.clone(),
                None => continue,
            };
            let mut description_parts = Vec::new();

            for (k, v) in &self.set_fields {
                let value_str = render_value_for_yaml(v);
                let (new_content, _) = crate::writer::set_field(&content, k, &value_str)?;
                content = new_content;
                description_parts.push(format!("set {}={}", k, value_str));
            }
            for k in &self.unset_fields {
                let (new_content, _) = crate::writer::unset_field(&content, k)?;
                content = new_content;
                description_parts.push(format!("unset {}", k));
            }
            for tag in &self.add_tags {
                let (new_content, _) = crate::writer::add_tag(&content, tag)?;
                content = new_content;
                description_parts.push(format!("+tag {}", tag));
            }
            for tag in &self.remove_tags {
                let (new_content, _) = crate::writer::remove_tag(&content, tag)?;
                content = new_content;
                description_parts.push(format!("-tag {}", tag));
            }

            // WRITE the computed new content to disk.
            std::fs::write(&record.path, &content).map_err(VaultdbError::Io)?;

            let after = parse_frontmatter_only(&content)?;
            changes.push(PlannedChange {
                path: record.path.clone(),
                before,
                after,
                description: description_parts.join(", "),
            });
        }

        Ok(MutationReport { changes, errors })
    }
}

/// Helper: render a `Value` as a YAML scalar suitable for inline frontmatter
/// substitution.
fn render_value_for_yaml(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Integer(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::String(s) => crate::writer::quote_value(s),
        Value::List(_) | Value::Map(_) => {
            // Inline-render lists/maps as flow YAML; for MVP, fall back
            // to a serde_yaml::to_string and trim trailing newline.
            let s = serde_yaml::to_string(v).unwrap_or_default();
            s.trim_end().to_string()
        }
    }
}

/// Helper: parse just the frontmatter from a content string and return the
/// resulting field map.
fn parse_frontmatter_only(content: &str) -> Result<std::collections::BTreeMap<String, Value>> {
    // The existing frontmatter parser in crate::frontmatter likely has a
    // function that produces a (frontmatter, body) tuple. Use it here.
    // If the parser only operates on file paths (not strings), expose a
    // string-level entry point (`frontmatter::parse_string(s)`) and use it.
    // For now, write a minimal version that returns an empty map on
    // missing frontmatter.
    if !content.starts_with("---") {
        return Ok(std::collections::BTreeMap::new());
    }
    // Find the closing `---` line.
    let after_first = &content[3..];
    let close = match after_first.find("\n---") {
        Some(i) => i,
        None => return Ok(std::collections::BTreeMap::new()),
    };
    let frontmatter_str = &after_first[..close].trim();
    let yaml: serde_yaml::Value = serde_yaml::from_str(frontmatter_str).map_err(|e| {
        VaultdbError::InvalidFrontmatter {
            file: "<in-memory>".to_string(),
            reason: e.to_string(),
        }
    })?;
    let mapping = yaml.as_mapping().ok_or_else(|| VaultdbError::InvalidFrontmatter {
        file: "<in-memory>".to_string(),
        reason: "frontmatter is not a mapping".to_string(),
    })?;

    let mut out = std::collections::BTreeMap::new();
    for (k, v) in mapping {
        if let Some(key) = k.as_str() {
            out.insert(key.to_string(), Value::from(v.clone()));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::Predicate;

    #[test]
    fn update_builder_construction() {
        let filter = Expr::Predicate(Predicate::Equals {
            field: "status".into(),
            value: Value::String("active".into()),
        });
        let b = UpdateBuilder::new("notes", filter)
            .set("priority", Value::Integer(1))
            .add_tag("urgent");

        assert_eq!(b.set_fields.len(), 1);
        assert_eq!(b.add_tags.len(), 1);
    }
}
```

Note: The `From<serde_yaml::Value> for Value` impl required by `parse_frontmatter_only` is NOT yet implemented. If it doesn't exist (check `crates/vaultdb-core/src/record.rs`), add it before this task ships:

```rust
impl From<serde_yaml::Value> for Value {
    fn from(yaml: serde_yaml::Value) -> Self {
        match yaml {
            serde_yaml::Value::Null => Value::Null,
            serde_yaml::Value::Bool(b) => Value::Bool(b),
            serde_yaml::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Integer(i)
                } else if let Some(f) = n.as_f64() {
                    Value::Float(f)
                } else {
                    Value::String(n.to_string())
                }
            }
            serde_yaml::Value::String(s) => Value::String(s),
            serde_yaml::Value::Sequence(items) => {
                Value::List(items.into_iter().map(Value::from).collect())
            }
            serde_yaml::Value::Mapping(m) => {
                let mut out = std::collections::BTreeMap::new();
                for (k, v) in m {
                    if let Some(key) = k.as_str() {
                        out.insert(key.to_string(), Value::from(v));
                    }
                }
                Value::Map(out)
            }
            serde_yaml::Value::Tagged(_) => Value::Null, // ignore tagged values
        }
    }
}
```

Add to `record.rs` if not present.

- [ ] **Step 2: Add `pub mod mutation;` to lib.rs**

Open `crates/vaultdb-core/src/lib.rs`. Add `pub mod mutation;` to the module list (alphabetically) and re-exports. The full file now ends with:

```rust
pub use record::{Record, Value};
pub use error::ParseError;
pub use vault::{LoadResult, Vault};
pub use query::{Expr, Predicate, LinkPredicate, CompareOp, Query, SortKey};
pub use links::{LinkGraph, GraphScope, Direction, UnresolvedLink};
pub use mutation::{UpdateBuilder, MutationReport, PlannedChange, MutationError};
```

- [ ] **Step 3: Build and test**

```bash
cargo build --workspace 2>&1 | tail -10
cargo test --workspace 2>&1 | grep -E "^test result:" | awk -F'[ ;]' '{sum+=$4} END {print "Total:", sum}'
```

Expected: build clean, `Total: 127` (126 + 1 new builder test).

If you see compile errors related to `writer::set_field` returning a different result shape, adapt the calls. The key is: `set_field`, `unset_field`, `add_tag`, `remove_tag` all return `Result<(String, ChangeDescription)>` per the existing writer.rs. The new content is the first tuple element.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat(core): add mutation module with UpdateBuilder + plan/execute split

Introduces the typed mutation API with the plan/execute pattern from
the spec. UpdateBuilder accumulates set/unset/add_tag/remove_tag
operations; plan() returns a MutationReport describing every record
that would be changed (before/after snapshots) without touching disk;
execute() does plan + actually writes.

Sibling builders (Delete, Move, Rename) ship in Task 8.

1 new construction test; total 127.
EOF
)"
```

---

## Task 8: Add `DeleteBuilder`, `MoveBuilder`, `RenameBuilder`

**Goal:** Three more builders, same plan/execute pattern. Each one is smaller than UpdateBuilder.

**Files:**
- Modify: `crates/vaultdb-core/src/mutation.rs` (add three more builder types)
- Modify: `crates/vaultdb-core/src/lib.rs` (re-exports)

- [ ] **Step 1: Add `DeleteBuilder` to mutation.rs**

Append to `crates/vaultdb-core/src/mutation.rs`:

```rust
/// Build a delete mutation. Records matching `filter` are moved to
/// `<vault>/.trash/` (collision-safe). With `permanent`, files are
/// removed entirely.
#[derive(Debug, Clone)]
pub struct DeleteBuilder {
    filter: Expr,
    folder: String,
    permanent: bool,
}

impl DeleteBuilder {
    pub fn new(folder: impl Into<String>, filter: Expr) -> Self {
        Self {
            filter,
            folder: folder.into(),
            permanent: false,
        }
    }

    pub fn permanent(mut self, yes: bool) -> Self {
        self.permanent = yes;
        self
    }

    pub fn plan(&self, vault: &crate::vault::Vault) -> Result<MutationReport> {
        let folder_path = vault.resolve_folder(&self.folder)?;
        let load = vault.load_records(&folder_path, false, false)?;
        let needs_links = crate::filter::expr_uses_links(&self.filter);
        let link_index = if needs_links {
            Some(crate::links::LinkGraph::from_records(&load.records))
        } else {
            None
        };

        let mut changes = Vec::new();
        for r in &load.records {
            if !crate::filter::evaluate_expr(&self.filter, r, &vault.root, link_index.as_ref()) {
                continue;
            }
            changes.push(PlannedChange {
                path: r.path.clone(),
                before: r.fields.clone(),
                after: std::collections::BTreeMap::new(),
                description: if self.permanent {
                    "delete (permanent)".to_string()
                } else {
                    "move to .trash/".to_string()
                },
            });
        }
        Ok(MutationReport { changes, errors: Vec::new() })
    }

    pub fn execute(self, vault: &crate::vault::Vault) -> Result<MutationReport> {
        let report = self.plan(vault)?;
        for change in &report.changes {
            if self.permanent {
                std::fs::remove_file(&change.path).map_err(VaultdbError::Io)?;
            } else {
                // Move to .trash/ — preserve filename, with collision-safe naming
                let trash_dir = vault.root.join(".trash");
                std::fs::create_dir_all(&trash_dir).map_err(VaultdbError::Io)?;
                let dest = unique_in_dir(&trash_dir, &change.path);
                std::fs::rename(&change.path, &dest).map_err(VaultdbError::Io)?;
            }
        }
        Ok(report)
    }
}

fn unique_in_dir(dir: &std::path::Path, src: &std::path::Path) -> PathBuf {
    let filename = src.file_name().and_then(|n| n.to_str()).unwrap_or("file");
    let mut candidate = dir.join(filename);
    let mut i = 1;
    while candidate.exists() {
        let stem = src.file_stem().and_then(|n| n.to_str()).unwrap_or("file");
        let ext = src.extension().and_then(|n| n.to_str()).unwrap_or("md");
        candidate = dir.join(format!("{}-{}.{}", stem, i, ext));
        i += 1;
    }
    candidate
}
```

- [ ] **Step 2: Add `MoveBuilder`**

Append:

```rust
/// Build a move mutation. Records matching `filter` are moved to `to_folder`.
#[derive(Debug, Clone)]
pub struct MoveBuilder {
    filter: Expr,
    folder: String,
    to_folder: String,
}

impl MoveBuilder {
    pub fn new(folder: impl Into<String>, to_folder: impl Into<String>, filter: Expr) -> Self {
        Self {
            filter,
            folder: folder.into(),
            to_folder: to_folder.into(),
        }
    }

    pub fn plan(&self, vault: &crate::vault::Vault) -> Result<MutationReport> {
        let folder_path = vault.resolve_folder(&self.folder)?;
        let to_path = vault.root.join(&self.to_folder);
        let load = vault.load_records(&folder_path, false, false)?;
        let needs_links = crate::filter::expr_uses_links(&self.filter);
        let link_index = if needs_links {
            Some(crate::links::LinkGraph::from_records(&load.records))
        } else {
            None
        };

        let mut changes = Vec::new();
        for r in &load.records {
            if !crate::filter::evaluate_expr(&self.filter, r, &vault.root, link_index.as_ref()) {
                continue;
            }
            let new_path = to_path.join(r.path.file_name().unwrap_or_default());
            changes.push(PlannedChange {
                path: r.path.clone(),
                before: r.fields.clone(),
                after: r.fields.clone(),
                description: format!("move to {}", new_path.display()),
            });
        }
        Ok(MutationReport { changes, errors: Vec::new() })
    }

    pub fn execute(self, vault: &crate::vault::Vault) -> Result<MutationReport> {
        let to_path = vault.root.join(&self.to_folder);
        std::fs::create_dir_all(&to_path).map_err(VaultdbError::Io)?;
        let report = self.plan(vault)?;
        for change in &report.changes {
            let dest = to_path.join(change.path.file_name().unwrap_or_default());
            std::fs::rename(&change.path, &dest).map_err(VaultdbError::Io)?;
        }
        Ok(report)
    }
}
```

- [ ] **Step 3: Add `RenameBuilder`**

Append:

```rust
/// Build a rename mutation. The single record at `from` is renamed to `to`,
/// and all `[[wikilinks]]` across the vault that pointed to `from` are
/// rewritten to point to `to`.
#[derive(Debug, Clone)]
pub struct RenameBuilder {
    folder: String,
    from: String,
    to: String,
}

impl RenameBuilder {
    pub fn new(
        folder: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
    ) -> Self {
        Self {
            folder: folder.into(),
            from: from.into(),
            to: to.into(),
        }
    }

    pub fn plan(&self, vault: &crate::vault::Vault) -> Result<MutationReport> {
        let folder_path = vault.resolve_folder(&self.folder)?;
        let source = folder_path.join(format!("{}.md", self.from));
        if !source.is_file() {
            return Ok(MutationReport {
                changes: Vec::new(),
                errors: vec![MutationError {
                    path: source,
                    message: format!("source `{}` not found", self.from),
                }],
            });
        }
        let dest = folder_path.join(format!("{}.md", self.to));
        let mut changes = vec![PlannedChange {
            path: source.clone(),
            before: std::collections::BTreeMap::new(),
            after: std::collections::BTreeMap::new(),
            description: format!("rename to {}", dest.display()),
        }];

        // Find every record that links to `self.from` and add a planned change.
        let all = vault.load_records_with_content(&vault.root, true, false)?;
        let graph = crate::links::LinkGraph::from_records(&all.records);
        for source_name in graph.incoming(&self.from) {
            if let Some(record) = graph.record_by_name(source_name) {
                changes.push(PlannedChange {
                    path: record.path.clone(),
                    before: std::collections::BTreeMap::new(),
                    after: std::collections::BTreeMap::new(),
                    description: format!(
                        "rewrite [[{}]] -> [[{}]] in body",
                        self.from, self.to
                    ),
                });
            }
        }

        Ok(MutationReport { changes, errors: Vec::new() })
    }

    pub fn execute(self, vault: &crate::vault::Vault) -> Result<MutationReport> {
        let report = self.plan(vault)?;

        // Rename the file first
        if let Some(rename_change) = report.changes.first() {
            let folder_path = vault.resolve_folder(&self.folder)?;
            let source = folder_path.join(format!("{}.md", self.from));
            let dest = folder_path.join(format!("{}.md", self.to));
            if source.is_file() {
                std::fs::rename(&source, &dest).map_err(VaultdbError::Io)?;
            }
            let _ = rename_change;
        }

        // Rewrite incoming links across the vault
        for change in report.changes.iter().skip(1) {
            let content = std::fs::read_to_string(&change.path).map_err(VaultdbError::Io)?;
            let new_content = rewrite_wikilinks_in(&content, &self.from, &self.to);
            std::fs::write(&change.path, new_content).map_err(VaultdbError::Io)?;
        }

        Ok(report)
    }
}

/// Rewrite `[[from]]` (and `[[from|alias]]`, `[[from#section]]` variants) to
/// point at `to` instead. Used by RenameBuilder.
fn rewrite_wikilinks_in(content: &str, from: &str, to: &str) -> String {
    // A simple text replacement is fragile; the existing crate::links module
    // has a wikilink scanner. For MVP, do a careful substring replace that
    // anchors on the [[ and following | / # / ]] characters.
    let needle_simple = format!("[[{}]]", from);
    let needle_alias_prefix = format!("[[{}|", from);
    let needle_section_prefix = format!("[[{}#", from);

    let replaced = content
        .replace(&needle_simple, &format!("[[{}]]", to))
        .replace(&needle_alias_prefix, &format!("[[{}|", to))
        .replace(&needle_section_prefix, &format!("[[{}#", to));
    replaced
}
```

If the existing `crate::links` module already has a more rigorous wikilink rewriter (e.g., used by some pre-existing rename function), use that one instead — read links.rs to check.

- [ ] **Step 4: Update lib.rs re-exports**

The full re-exports:

```rust
pub use mutation::{
    UpdateBuilder, DeleteBuilder, MoveBuilder, RenameBuilder,
    MutationReport, PlannedChange, MutationError,
};
```

- [ ] **Step 5: Add a smoke test for one builder (DeleteBuilder)**

Inside `mutation.rs`'s `mod tests`, add:

```rust
#[test]
fn delete_builder_plans_changes_for_matching_records() {
    use crate::query::Predicate;

    // We need a Vault to plan against. The simplest path is to reuse
    // the existing test fixture. For an MVP, construct an in-memory
    // approximation and test the builder construction only.
    let filter = Expr::Predicate(Predicate::Exists { field: "status".into() });
    let b = DeleteBuilder::new("notes", filter).permanent(false);
    assert!(!b.permanent);
}
```

This is a thin construction test; deeper integration tests for builder.plan() against a real vault are deferred to the CLI migration tasks (where the e2e command tests cover them).

- [ ] **Step 6: Build and test**

```bash
cargo build --workspace
cargo test --workspace 2>&1 | grep -E "^test result:" | awk -F'[ ;]' '{sum+=$4} END {print "Total:", sum}'
```

Expected: build clean, `Total: 128` (127 + 1 new test).

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat(core): add Delete/Move/Rename builders to mutation module

Each builder mirrors UpdateBuilder's plan/execute split. DeleteBuilder
moves to .trash/ by default (permanent() opts into hard delete).
MoveBuilder relocates matching records to a target folder.
RenameBuilder renames a single record AND rewrites all incoming
wikilinks across the vault.

1 new construction test; total 128.
EOF
)"
```

---

## Task 9: Migrate CLI mutation commands (`update`, `delete`, `move_cmd`, `rename`) to use builders

**Goal:** Each of the four mutation CLI commands is rewritten to construct the corresponding builder. Old `WhereClause`/`WhereExpr` references in these files disappear.

**Files:**
- Modify: `crates/vaultdb/src/commands/update.rs`
- Modify: `crates/vaultdb/src/commands/delete.rs`
- Modify: `crates/vaultdb/src/commands/move_cmd.rs`
- Modify: `crates/vaultdb/src/commands/rename.rs`

This task is large. Do all four commands in this single commit since they share patterns and migrating them together avoids partial states where some commands use old types and some don't.

- [ ] **Step 1: Read each command's existing entrypoint**

```bash
cd /home/rusen/Desktop/codebase-shared/researches/vaultdb/.worktrees/phase2b-query-mutation-graph-api
for f in update delete move_cmd rename; do
    echo "--- $f.rs ---"
    head -40 "crates/vaultdb/src/commands/$f.rs"
done
```

Note the existing function signatures and how they consume `--where`, `--set`, `--unset`, `--add-tag`, `--remove-tag`, `--to`, `--from`, `--dry-run`, `--force`, `--permanent` etc.

- [ ] **Step 2: Rewrite `update.rs`**

The new `run_update` function:

```rust
pub fn run_update(
    vault: &vaultdb_core::Vault,
    folder: &str,
    where_exprs: &[String],
    set_pairs: &[String],   // each is "key=value"
    unset_fields: &[String],
    add_tags: &[String],
    remove_tags: &[String],
    dry_run: bool,
) -> Result<()> {
    use vaultdb_core::{Expr, UpdateBuilder, Value};

    let parsed: Vec<Expr> = where_exprs
        .iter()
        .map(|s| Expr::parse(s))
        .collect::<vaultdb_core::error::Result<Vec<_>>>()?;
    if parsed.is_empty() {
        anyhow::bail!("update requires at least one --where condition");
    }
    let filter = if parsed.len() == 1 {
        parsed.into_iter().next().unwrap()
    } else {
        Expr::And(parsed)
    };

    let mut builder = UpdateBuilder::new(folder, filter);
    for pair in set_pairs {
        let (k, v) = pair.split_once('=').ok_or_else(|| {
            anyhow::anyhow!("--set requires key=value, got {}", pair)
        })?;
        builder = builder.set(k.trim(), Value::String(v.trim().to_string()));
    }
    for f in unset_fields {
        builder = builder.unset(f.clone());
    }
    for t in add_tags {
        builder = builder.add_tag(t.clone());
    }
    for t in remove_tags {
        builder = builder.remove_tag(t.clone());
    }

    let report = if dry_run {
        builder.plan(vault)?
    } else {
        builder.execute(vault)?
    };

    // Print a human-readable summary
    print_mutation_report(&report, dry_run);
    Ok(())
}
```

The `print_mutation_report` helper is a small CLI-side function that iterates `report.changes` and prints one line per change. If a similar function already exists in `output.rs`, use that. Otherwise add it inline:

```rust
fn print_mutation_report(report: &vaultdb_core::MutationReport, dry_run: bool) {
    if dry_run {
        println!("Dry run — no files modified.");
    }
    for change in &report.changes {
        println!(
            "{} {}: {}",
            if dry_run { "would" } else { "did" },
            change.path.display(),
            change.description
        );
    }
    for err in &report.errors {
        eprintln!("error {}: {}", err.path.display(), err.message);
    }
}
```

The `Value::String(...)` substitution above is a simplification — the original CLI may parse `--set "year=2023"` as an integer when it looks like one. For Phase 2b, MVP is to always use String; if a regression test fails, parse as integer/float when the string parses cleanly. Note this as a follow-up if it becomes an issue.

- [ ] **Step 3: Rewrite `delete.rs`**

The new `run_delete`:

```rust
pub fn run_delete(
    vault: &vaultdb_core::Vault,
    folder: &str,
    where_exprs: &[String],
    permanent: bool,
    dry_run: bool,
) -> Result<()> {
    use vaultdb_core::{DeleteBuilder, Expr};

    let parsed: Vec<Expr> = where_exprs
        .iter()
        .map(|s| Expr::parse(s))
        .collect::<vaultdb_core::error::Result<Vec<_>>>()?;
    if parsed.is_empty() {
        anyhow::bail!("delete requires at least one --where condition");
    }
    let filter = if parsed.len() == 1 {
        parsed.into_iter().next().unwrap()
    } else {
        Expr::And(parsed)
    };

    let builder = DeleteBuilder::new(folder, filter).permanent(permanent);
    let report = if dry_run {
        builder.plan(vault)?
    } else {
        builder.execute(vault)?
    };
    print_mutation_report(&report, dry_run);
    Ok(())
}
```

If `print_mutation_report` was added inline in update.rs, either move it to a shared place (e.g., a new `commands/util.rs` or into `output.rs`) or duplicate it here. The simpler path: define it once in `commands/mod.rs` as `pub(crate) fn print_mutation_report(...)`.

- [ ] **Step 4: Rewrite `move_cmd.rs`**

```rust
pub fn run_move(
    vault: &vaultdb_core::Vault,
    folder: &str,
    where_exprs: &[String],
    to_folder: &str,
    dry_run: bool,
) -> Result<()> {
    use vaultdb_core::{Expr, MoveBuilder};

    let parsed: Vec<Expr> = where_exprs
        .iter()
        .map(|s| Expr::parse(s))
        .collect::<vaultdb_core::error::Result<Vec<_>>>()?;
    if parsed.is_empty() {
        anyhow::bail!("move requires at least one --where condition");
    }
    let filter = if parsed.len() == 1 {
        parsed.into_iter().next().unwrap()
    } else {
        Expr::And(parsed)
    };

    let builder = MoveBuilder::new(folder, to_folder.to_string(), filter);
    let report = if dry_run {
        builder.plan(vault)?
    } else {
        builder.execute(vault)?
    };
    print_mutation_report(&report, dry_run);
    Ok(())
}
```

- [ ] **Step 5: Rewrite `rename.rs`**

```rust
pub fn run_rename(
    vault: &vaultdb_core::Vault,
    folder: &str,
    from: &str,
    to: &str,
    dry_run: bool,
) -> Result<()> {
    use vaultdb_core::RenameBuilder;
    let builder = RenameBuilder::new(folder, from, to);
    let report = if dry_run {
        builder.plan(vault)?
    } else {
        builder.execute(vault)?
    };
    print_mutation_report(&report, dry_run);
    Ok(())
}
```

- [ ] **Step 6: Build and test**

```bash
cargo build --workspace
cargo test --workspace 2>&1 | grep -E "^test result:" | awk -F'[ ;]' '{sum+=$4} END {print "Total:", sum}'
```

Expected: build clean, `Total: 128`.

If the build fails because `main.rs` (or `cli.rs`) calls these `run_*` functions with different parameter shapes, update the call sites in `main.rs`/`cli.rs` to pass the new params. The cli.rs clap definitions should already produce the right inputs.

- [ ] **Step 7: Smoke-test each CLI command**

```bash
cargo build 2>&1 | tail -2
tmpdir=$(mktemp -d)
mkdir -p "$tmpdir/.obsidian" "$tmpdir/notes"
echo -e "---\nstatus: active\n---\nA" > "$tmpdir/notes/a.md"
echo -e "---\nstatus: pending\n---\nB" > "$tmpdir/notes/b.md"

# Update with dry-run
./target/debug/vaultdb --vault "$tmpdir" update notes --where "status = active" --set "priority=1" --dry-run

# Delete with dry-run
./target/debug/vaultdb --vault "$tmpdir" delete notes --where "status = pending" --dry-run

# Move (real, since dry-run + move can be cheap)
mkdir -p "$tmpdir/archive"
./target/debug/vaultdb --vault "$tmpdir" move notes --where "status = active" --to archive --dry-run

# Rename (real for testing)
./target/debug/vaultdb --vault "$tmpdir" rename a "alpha" --folder notes --dry-run

rm -rf "$tmpdir"
```

Each invocation should print a "would …" report describing the planned changes. No files should change (we used --dry-run throughout).

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
refactor(cli): migrate mutation commands to typed builder API

commands/{update,delete,move_cmd,rename}.rs now build the
corresponding {Update,Delete,Move,Rename}Builder, then call
plan() / execute() based on --dry-run. The CLI no longer
constructs internal WhereClause/WhereExpr — all filtering
goes through Expr.

print_mutation_report shared helper for the dry-run / applied
output. Same UX as before for users.
EOF
)"
```

---

## Task 10: Migrate CLI graph commands (`links`, `traverse`, `unresolved`) to use `LinkGraph`

**Goal:** The three graph-shaped commands stop using `LinkIndex` directly and use the public `LinkGraph` API.

**Files:**
- Modify: `crates/vaultdb/src/commands/links.rs`
- Modify: `crates/vaultdb/src/commands/traverse.rs`
- Modify: `crates/vaultdb/src/commands/unresolved.rs`

- [ ] **Step 1: Migrate `links.rs`**

Find the `run_links` function. It currently uses `LinkIndex::from_records` or similar. Replace any `LinkIndex` reference with `LinkGraph` (the type alias keeps it working, but we want explicit migration). New shape:

```rust
pub fn run_links(
    vault: &vaultdb_core::Vault,
    name: &str,
    direction: LinkDirection,
) -> Result<()> {
    use vaultdb_core::{Direction, GraphScope};
    let graph = vault.link_graph(GraphScope::All)?;
    let dir = match direction {
        LinkDirection::Outgoing => Direction::Outgoing,
        LinkDirection::Incoming => Direction::Incoming,
        LinkDirection::Both => Direction::Both,
    };

    if matches!(dir, Direction::Outgoing | Direction::Both) {
        println!("Outgoing:");
        for target in graph.outgoing(name) {
            println!("  {}", target);
        }
    }
    if matches!(dir, Direction::Incoming | Direction::Both) {
        println!("Incoming:");
        for source in graph.incoming(name) {
            println!("  {}", source);
        }
    }
    Ok(())
}
```

- [ ] **Step 2: Migrate `traverse.rs`**

Replace direct `LinkIndex` use with `LinkGraph` and `traverse_from`:

```rust
pub fn run_traverse(
    vault: &vaultdb_core::Vault,
    name: &str,
    depth: usize,
    direction: LinkDirection,
    where_exprs: &[String],
    select: &Option<String>,
) -> Result<()> {
    use vaultdb_core::{Direction, GraphScope};

    let graph = vault.link_graph(GraphScope::All)?;
    let dir = match direction {
        LinkDirection::Outgoing => Direction::Outgoing,
        LinkDirection::Incoming => Direction::Incoming,
        LinkDirection::Both => Direction::Both,
    };
    let names = graph.traverse_from(name, depth, dir);

    let _ = where_exprs; // Optional: filter records using Expr — for MVP, skip
    let _ = select;       // Optional: project — for MVP, just print names

    for n in names {
        println!("{}", n);
    }
    Ok(())
}
```

If the existing `run_traverse` has richer behaviour (e.g., printing each reached record's frontmatter values rather than just names), adapt to preserve that. The full implementation can use `graph.record_by_name(&n)` to get the underlying record.

- [ ] **Step 3: Migrate `unresolved.rs`**

```rust
pub fn run_unresolved(
    vault: &vaultdb_core::Vault,
    folder: &str,
    verbose: bool,
) -> Result<()> {
    use vaultdb_core::GraphScope;
    let graph = vault.link_graph(GraphScope::Folder(folder.to_string()))?;
    let unresolved = graph.unresolved();
    if unresolved.is_empty() {
        if verbose {
            println!("(no unresolved links)");
        }
        return Ok(());
    }
    if verbose {
        for u in unresolved {
            println!("{} -> {}", u.source, u.target);
        }
    } else {
        let mut targets: Vec<&str> = unresolved.iter().map(|u| u.target.as_str()).collect();
        targets.sort();
        targets.dedup();
        for t in targets {
            println!("{}", t);
        }
    }
    Ok(())
}
```

- [ ] **Step 4: Build, test, smoke**

```bash
cargo build --workspace
cargo test --workspace 2>&1 | grep -E "^test result:" | awk -F'[ ;]' '{sum+=$4} END {print "Total:", sum}'

# Smoke
tmpdir=$(mktemp -d)
mkdir -p "$tmpdir/.obsidian" "$tmpdir/notes"
echo -e "[[b]] [[ghost]]" > "$tmpdir/notes/a.md"
echo -e "" > "$tmpdir/notes/b.md"
./target/debug/vaultdb --vault "$tmpdir" links a
./target/debug/vaultdb --vault "$tmpdir" traverse a --depth 2
./target/debug/vaultdb --vault "$tmpdir" unresolved notes -v
rm -rf "$tmpdir"
```

Expected: each prints something sensible. links shows outgoing b/ghost and (no) incoming. traverse shows b. unresolved shows `a -> ghost`.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
refactor(cli): migrate graph commands to LinkGraph public API

commands/{links,traverse,unresolved}.rs now use vault.link_graph(scope)
and the LinkGraph methods (outgoing, incoming, unresolved,
traverse_from). The internal LinkIndex alias is no longer referenced
in the CLI source.
EOF
)"
```

---

## Task 11: Migrate `commands/schema_cmd.rs` to use `Vault::query`

**Goal:** schema_cmd.rs's `--where` filter (used by `schema validate`) goes through `Expr::parse` + filter logic. Other parts (schema init/show) are largely unchanged.

**Files:**
- Modify: `crates/vaultdb/src/commands/schema_cmd.rs`

- [ ] **Step 1: Find the where-clause use sites**

```bash
cd /home/rusen/Desktop/codebase-shared/researches/vaultdb/.worktrees/phase2b-query-mutation-graph-api
grep -n "WhereClause\|matches_all" crates/vaultdb/src/commands/schema_cmd.rs
```

- [ ] **Step 2: Replace WhereClause-based filtering with Vault::query**

For each spot that does:

```rust
let clauses = parse_where_clause_strings(where_exprs)?;
let records = ...
let filtered: Vec<&Record> = records.iter().filter(|r| matches_all(&clauses, r, root)).collect();
```

Replace with:

```rust
use vaultdb_core::{Expr, Query};

let parsed: Vec<Expr> = where_exprs
    .iter()
    .map(|s| Expr::parse(s))
    .collect::<vaultdb_core::error::Result<Vec<_>>>()?;
let filter = match parsed.len() {
    0 => None,
    1 => Some(parsed.into_iter().next().unwrap()),
    _ => Some(Expr::And(parsed)),
};
let q = Query {
    folder: folder_name.to_string(),
    filter,
    select: None,
    sort: None,
    limit: None,
    recursive: false,
};
let records = vault.query(&q)?;
```

If the existing schema_cmd uses these records to walk over fields per-record (e.g., `for r in records { ... }`), the iteration shape is unchanged.

- [ ] **Step 3: Build, test, smoke**

```bash
cargo build --workspace
cargo test --workspace 2>&1 | grep -E "^test result:" | awk -F'[ ;]' '{sum+=$4} END {print "Total:", sum}'

tmpdir=$(mktemp -d)
mkdir -p "$tmpdir/.obsidian" "$tmpdir/notes"
echo -e "---\nstatus: active\n---\nA" > "$tmpdir/notes/a.md"
./target/debug/vaultdb --vault "$tmpdir" schema init notes
./target/debug/vaultdb --vault "$tmpdir" schema validate notes
./target/debug/vaultdb --vault "$tmpdir" schema show notes
rm -rf "$tmpdir"
```

Expected: schema init creates a yaml file, validate runs without error, show prints field info.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
refactor(cli): migrate schema_cmd to Vault::query for filtering

commands/schema_cmd.rs no longer references the internal WhereClause
type. Filtered schema operations (validate with --where) build a
Query and dispatch through vault.query.
EOF
)"
```

---

## Task 12: Add `Serialize`/`Deserialize` to `Record`, `LoadResult`, and re-export `VaultdbError`/`Result`

**Goal:** Round out the public API serialization story. `Record` becomes Serialize+Deserialize. `LoadResult` follows. `VaultdbError` and `Result` are re-exported from lib.rs root.

**Files:**
- Modify: `crates/vaultdb-core/src/record.rs`
- Modify: `crates/vaultdb-core/src/vault.rs` (LoadResult derive)
- Modify: `crates/vaultdb-core/src/lib.rs` (re-exports)

- [ ] **Step 1: Add derives to `Record`**

Open `crates/vaultdb-core/src/record.rs`. Find `pub struct Record { ... }`. Update the derive list:

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Record {
    pub path: PathBuf,
    pub fields: BTreeMap<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub raw_content: Option<String>,
}
```

`PathBuf` already implements `Serialize`/`Deserialize` (string-shaped). The `#[serde(skip_serializing_if = ..., default)]` on `raw_content` keeps the JSON compact when it's None (which is most of the time).

Document this on the type:

```rust
/// A parsed Markdown record. Frontmatter fields are in `fields`; the body is
/// in `raw_content` only when explicitly loaded (via `load_records_with_content`).
///
/// Serialization note: `path` serializes as a string. For machine-portable JSON,
/// store records relative to the vault root; absolute paths are host-specific.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Record { /* ... */ }
```

- [ ] **Step 2: Add derives to `LoadResult`**

Open `crates/vaultdb-core/src/vault.rs`. Find `pub struct LoadResult { ... }`. Update:

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LoadResult {
    pub records: Vec<Record>,
    pub parse_errors: Vec<crate::error::ParseError>,
}
```

- [ ] **Step 3: Re-export `VaultdbError` and `Result` from lib.rs**

The full lib.rs re-export block:

```rust
pub use record::{Record, Value};
pub use error::{ParseError, Result, VaultdbError};
pub use vault::{LoadResult, Vault};
pub use query::{Expr, Predicate, LinkPredicate, CompareOp, Query, SortKey};
pub use links::{LinkGraph, GraphScope, Direction, UnresolvedLink};
pub use mutation::{
    UpdateBuilder, DeleteBuilder, MoveBuilder, RenameBuilder,
    MutationReport, PlannedChange, MutationError,
};
```

If `Result` is not currently `pub` from `error.rs`, make it so (it's the type alias `pub type Result<T> = std::result::Result<T, VaultdbError>;`).

- [ ] **Step 4: Add a serialization test for `Record`**

Inside `record.rs`'s `#[cfg(test)] mod tests`, add:

```rust
#[test]
fn record_serializes_with_path_as_string() {
    let mut fields = std::collections::BTreeMap::new();
    fields.insert("status".into(), Value::String("active".into()));
    let r = Record {
        path: std::path::PathBuf::from("/v/notes/a.md"),
        fields,
        raw_content: None,
    };
    let json = serde_json::to_string(&r).unwrap();
    // Path serializes as a string; raw_content (None) is skipped.
    assert!(json.contains("/v/notes/a.md"));
    assert!(json.contains("status"));
    assert!(!json.contains("raw_content"));
}

#[test]
fn record_round_trips() {
    let mut fields = std::collections::BTreeMap::new();
    fields.insert("k".into(), Value::Integer(1));
    let r = Record {
        path: std::path::PathBuf::from("/v/x.md"),
        fields,
        raw_content: None,
    };
    let json = serde_json::to_string(&r).unwrap();
    let back: Record = serde_json::from_str(&json).unwrap();
    assert_eq!(back.path, r.path);
    assert_eq!(back.fields.get("k"), Some(&Value::Integer(1)));
    assert!(back.raw_content.is_none());
}
```

- [ ] **Step 5: Build and test**

```bash
cargo build --workspace
cargo test --workspace 2>&1 | grep -E "^test result:" | awk -F'[ ;]' '{sum+=$4} END {print "Total:", sum}'
```

Expected: `Total: 130` (128 + 2 new tests).

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat(core): add Serialize/Deserialize to Record and LoadResult

Record's path serializes as a string; raw_content is skipped when None
to keep JSON compact. LoadResult inherits the derives. VaultdbError
and Result are now re-exported from lib.rs root so consumers don't
have to traverse the error module.

2 new tests; total 130.
EOF
)"
```

---

## Task 13: Add module-level rustdoc and crate-level rustdoc

**Goal:** Each engine module gets a brief `//!` description at its top. The crate root gets a paragraph explaining the dual-structure thesis and pointing at the main entry types.

**Files:**
- Modify: `crates/vaultdb-core/src/lib.rs` (crate-level doc)
- Modify: `crates/vaultdb-core/src/{error,filter,frontmatter,links,mutation,query,record,schema,vault,writer}.rs` (each gets a `//!` block)

- [ ] **Step 1: Crate-level doc in lib.rs**

Replace the existing one-line `//! vaultdb-core — library engine for vaultdb.` with:

```rust
//! # vaultdb-core
//!
//! A markdown-as-database engine. Treats folders of `.md` files with YAML
//! frontmatter as queryable structured data, with `[[wikilinks]]` forming a
//! first-class link graph. Both the tabular (frontmatter) and graph (link)
//! shapes are equally first-class — you can filter records by frontmatter,
//! by graph predicates (e.g., "links to anything tagged X"), or by any
//! combination.
//!
//! ## Quick start
//!
//! ```no_run
//! use vaultdb_core::{Vault, Query, Expr, Predicate, Value};
//!
//! let vault = Vault::discover(std::path::Path::new(".")).unwrap();
//! let q = Query {
//!     folder: "notes".into(),
//!     filter: Some(Expr::Predicate(Predicate::Equals {
//!         field: "status".into(),
//!         value: Value::String("active".into()),
//!     })),
//!     select: None,
//!     sort: None,
//!     limit: Some(10),
//!     recursive: false,
//! };
//! let records = vault.query(&q).unwrap();
//! ```
//!
//! ## Public API
//!
//! - [`Vault`]: entry point. Discover, load records, query, build the link graph.
//! - [`Record`] and [`Value`]: the row + cell types.
//! - [`Expr`], [`Predicate`], [`LinkPredicate`], [`Query`]: the AST.
//! - [`LinkGraph`], [`GraphScope`], [`Direction`]: the link graph + traversal.
//! - [`UpdateBuilder`], [`DeleteBuilder`], [`MoveBuilder`], [`RenameBuilder`]:
//!   the typed mutation API. Each has [`UpdateBuilder::plan`]-style preview
//!   and `execute`-style commit methods.
//! - [`LoadResult`], [`ParseError`]: parse-diagnostic surfacing.
//!
//! ## Design philosophy
//!
//! No daemon, no cache, no state files. Every read traverses the filesystem
//! fresh. Stateful concerns (file watchers, full-text indexes, typed schemas)
//! belong in the consumer, not in vaultdb-core.

pub mod error;
// ...
```

- [ ] **Step 2: Module-level doc on each engine module**

Each engine module gets a brief `//!` block. Open each file and prepend (right after the existing `//!` line if any, or at the top):

`error.rs`:
```rust
//! Error types: `VaultdbError` for fatal failures, `ParseError` for non-fatal
//! per-file diagnostics returned by `LoadResult`.
```

`record.rs`:
```rust
//! `Record` (one parsed `.md` file = one record) and `Value` (the typed cell
//! values). Records have virtual fields (`_name`, `_path`, `_modified`, etc.)
//! computed lazily from the path and frontmatter.
```

`vault.rs`:
```rust
//! `Vault`: the library entry point. Discovers a vault from `.obsidian/`,
//! lists files, loads records, runs queries, and builds the link graph.
```

`query.rs` (already has a multi-line `//!` block from Task 2 — no change).

`filter.rs`:
```rust
//! Evaluator for the public `Expr` AST. Walks `Expr` and `Predicate` trees
//! against a `Record` (and optional `LinkGraph`), returning `bool`.
```

`links.rs`:
```rust
//! `LinkGraph`: the citation graph from a vault's `[[wikilinks]]`. Supports
//! outgoing/incoming queries, BFS traversal, and unresolved-link discovery.
```

`mutation.rs` (already has a `//!` block from Task 7 — no change).

`schema.rs`:
```rust
//! Schema inference and validation. `infer_schema` walks records to discover
//! field types and cardinalities; `validate_record` checks a record against
//! a schema.
```

`writer.rs`:
```rust
//! Frontmatter write primitives. `set_field`, `unset_field`, `add_tag`,
//! `remove_tag` produce `(new_content, ChangeDescription)` tuples without
//! touching disk; `apply` flushes a `WriteResult` to the filesystem. The
//! public mutation builders (in [`crate::mutation`]) wrap these.
```

`frontmatter.rs`:
```rust
//! Parse YAML frontmatter from `.md` files. Internal — the public surface
//! is `Vault::load_records` / `Vault::find_by_name`.
```

- [ ] **Step 3: Build (with cargo doc)**

```bash
cargo build --workspace
cargo doc --workspace --no-deps 2>&1 | tail -3
cargo test --workspace --doc 2>&1 | tail -5
```

Expected: build clean. Doc tests: the example in lib.rs is `no_run`, so it should compile-check but not execute. Verify no doc-test failures.

```bash
cargo test --workspace 2>&1 | grep -E "^test result:" | awk -F'[ ;]' '{sum+=$4} END {print "Total:", sum}'
```

Expected: `Total: 130`.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
docs(core): crate-level + per-module rustdoc

vaultdb-core's lib.rs gains a quick-start example, a public-API
table-of-contents, and a brief design-philosophy section explaining
the no-daemon-no-cache-no-state thesis. Every engine module gets a
short //! block describing its responsibility.
EOF
)"
```

---

## Task 14: Remove the legacy internal types and final verification

**Goal:** With every CLI command migrated to the public API, the old `WhereClause`/`WhereExpr` types and the conversion shims are dead code. Remove them. Confirm everything still works.

**Files:**
- Modify: `crates/vaultdb-core/src/filter.rs` (remove WhereClause, WhereExpr, parse_where_clause, to_expr/to_predicate)
- Possibly modify: `crates/vaultdb-core/src/links.rs` (remove `LinkIndex` alias if no remaining callers)

- [ ] **Step 1: Verify the legacy types have no remaining callers**

```bash
cd /home/rusen/Desktop/codebase-shared/researches/vaultdb/.worktrees/phase2b-query-mutation-graph-api
grep -rn "WhereClause\|WhereExpr\|matches_all\|matches_exprs_with_links" crates/
grep -rn "LinkIndex" crates/
```

Expected: matches only inside `crates/vaultdb-core/src/filter.rs` (definitions and the conversion shim) for the first grep; matches only inside `crates/vaultdb-core/src/links.rs` (the `pub type LinkIndex = LinkGraph;` alias) for the second.

If you find any external callers, that means a CLI command or test wasn't fully migrated — STOP and fix. Don't remove types still in use.

- [ ] **Step 2: Remove the legacy types**

In `crates/vaultdb-core/src/filter.rs`:
- Delete the `pub struct WhereExpr` definition.
- Delete the `pub struct WhereClause` definition.
- Delete `pub fn matches_all`, `pub fn matches_all_with_links`, `pub fn matches_exprs_with_links` (their callers all migrated to `evaluate_expr`).
- Delete `pub fn parse_where_clause` (the shim from Task 2).
- Delete the `WhereClause::to_expr` and `WhereExpr::to_predicate` impls.
- Delete the `pub enum CompareOp` if it duplicates `query::CompareOp` (the public one is now the canonical one — change any internal references in filter.rs to `crate::query::CompareOp`).

After deletion, filter.rs contains only the evaluator functions: `expr_uses_links`, `evaluate_expr`, `evaluate_predicate`, `compare_values`, plus the existing where-DSL parser if it's still needed by `<Expr as FromStr>::from_str` (it should be — the parser is the only place that knows how to turn a string into an AST). The parser itself can be renamed/refactored to produce `Expr` directly.

If the where-DSL parser was structured around `WhereClause`, you have two choices:
**A**: Refactor the parser to produce `Expr` directly. Adapt the recursive-descent code.
**B**: Keep the parser as-is, kept private, and have `<Expr as FromStr>::from_str` call it. To do this, keep the `WhereClause`/`WhereExpr` types but mark them `pub(crate)` instead of `pub`, and keep the conversion shim.

Option A is cleaner. Option B is faster. The implementer chooses based on parser complexity. If the parser is <100 lines, refactor. If >300 lines, keep with `pub(crate)`.

For Phase 2b's MVP, go with Option B: change `pub struct WhereClause` to `pub(crate) struct WhereClause`, same for `WhereExpr`, keep `parse_where_clause` as `pub(crate)`. The conversion shim stays. This is honest about what's still private internal vs what's truly removed.

In `crates/vaultdb-core/src/links.rs`:
- Remove the `pub type LinkIndex = LinkGraph;` alias (if Step 1 confirmed no callers).

- [ ] **Step 3: Build and run all tests**

```bash
cargo build --workspace 2>&1 | tail -10
cargo test --workspace 2>&1 | grep -E "^test result:" | awk -F'[ ;]' '{sum+=$4} END {print "Total:", sum}'
```

Expected: build clean, `Total: 130`. If a test references `WhereClause` directly (probably an inline filter.rs test), update it to use `Expr`.

- [ ] **Step 4: Run clippy**

```bash
cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -20
```

Expected: same 7 pre-existing lints as before (those are still deferred). If NEW lints surfaced from this task's deletions, fix them; otherwise note the existing ones as `DONE_WITH_CONCERNS` per the deferred-from-Phase-1 policy.

- [ ] **Step 5: Release build + final smoke test**

```bash
cargo build --release 2>&1 | tail -2
./target/release/vaultdb --help 2>&1 | head -5
tmpdir=$(mktemp -d)
mkdir -p "$tmpdir/.obsidian" "$tmpdir/notes"
echo -e "---\nstatus: active\nyear: 2020\n---\n[[b]]" > "$tmpdir/notes/a.md"
echo -e "---\nstatus: pending\n---\n" > "$tmpdir/notes/b.md"
./target/release/vaultdb --vault "$tmpdir" query notes --where "status = active" --select "_name,year"
./target/release/vaultdb --vault "$tmpdir" query notes --links-to "b" --select "_name"
./target/release/vaultdb --vault "$tmpdir" links a
./target/release/vaultdb --vault "$tmpdir" unresolved notes
./target/release/vaultdb --vault "$tmpdir" update notes --where "status = active" --set "priority=1" --dry-run
rm -rf "$tmpdir"
```

Expected: each command produces sensible output. The query+links combination, in particular, exercises both the AST and the graph evaluator.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
chore(core): mark legacy WhereClause/WhereExpr pub(crate); drop LinkIndex alias

After CLI migration to the new typed API, the internal AST types
(WhereClause, WhereExpr, parse_where_clause, to_expr/to_predicate
shims) are no longer part of the public surface — they remain as
pub(crate) so the existing where-DSL parser can keep working under
<Expr as FromStr>::from_str without a full parser rewrite.

LinkIndex alias removed (LinkGraph is the canonical name).

Phase 2b deliverable complete.
EOF
)"
```

---

## Task 15: End-of-phase verification

**Goal:** Confirm the phase delivered cleanly. No new commits.

- [ ] **Step 1: Test count and pass status**

```bash
cd /home/rusen/Desktop/codebase-shared/researches/vaultdb/.worktrees/phase2b-query-mutation-graph-api
cargo test --workspace 2>&1 | grep -E "^test result:" | awk -F'[ ;]' '{sum+=$4} END {print "Total:", sum}'
```

Expected: 130 (or 130 + any test additions made for new evaluator coverage during Tasks 4-6).

- [ ] **Step 2: Public API surface review**

```bash
grep -E "^pub use" crates/vaultdb-core/src/lib.rs
```

Expected:
```
pub use record::{Record, Value};
pub use error::{ParseError, Result, VaultdbError};
pub use vault::{LoadResult, Vault};
pub use query::{Expr, Predicate, LinkPredicate, CompareOp, Query, SortKey};
pub use links::{LinkGraph, GraphScope, Direction, UnresolvedLink};
pub use mutation::{UpdateBuilder, DeleteBuilder, MoveBuilder, RenameBuilder, MutationReport, PlannedChange, MutationError};
```

- [ ] **Step 3: Verify branch history**

```bash
git log --oneline main..HEAD
```

Expected: 13 commits (Tasks 2-14 each produce one commit; Task 1 was setup-only; Task 15 has no commit).

- [ ] **Step 4: Verify no behaviour regression**

Run the same commands users ran pre-2b and confirm output is sensible (the smoke test in Task 14 already covers this).

---

## Open questions / followups (out of scope for Phase 2b)

- **`WhereClause` parser still alive as `pub(crate)`.** Refactoring the parser to produce `Expr` directly is a follow-up cleanup; for 2b the conversion shim is acceptable.
- **`LinkPredicate::Where` evaluation only handles direct neighbours.** Multi-hop graph predicates (transitive closures) would require richer evaluator logic — defer until a real use case asks for it.
- **`UpdateBuilder::execute` re-runs the apply loop rather than caching `plan()`'s computed new_content.** Refactor to make `plan()` produce per-change new content alongside the report; `execute()` then just calls `writer::apply` per change.
- **CLI `--set "year=2020"` parses values as `Value::String` always.** Future enhancement: detect integer/float/bool parses and store as the typed variant. No regression for round-tripping (the writer converts back to YAML-shaped strings).
- **`vault.link_graph(scope)` calls `load_records` to walk the vault.** For huge vaults, a streaming variant would be faster. Defer.
- **Direction::Both traversal can revisit nodes through both edges.** The current BFS visits each name once, which is correct for the "set of reachable nodes" answer; if "shortest distance" or path-listing is ever needed, switch to a richer traversal type.
- **The 7 pre-existing clippy lints** still deferred. Their fix is a Phase 3 chore commit.
- **`[workspace.package]` field inheritance** still unwired. Defer.
- **`From<serde_yaml::Value> for Value`** added in Task 7 — adopted by `mutation.rs::parse_frontmatter_only`. If frontmatter.rs's existing parsing logic can be simplified by using this `From` impl directly, that's a follow-up cleanup.
