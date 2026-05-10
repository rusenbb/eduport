# Eduport — vaultdb-Backed Rewrite Design Spec

**Status:** draft for review
**Date:** 2026-05-09
**Author:** brainstormed with Claude
**Relates to:** [2026-05-06-eduport-design.md](2026-05-06-eduport-design.md),
[2026-05-09-custom-fields-design.md](2026-05-09-custom-fields-design.md)

---

## 1. Overview

Eduport's current implementation reinvents — in a Python sidecar — capabilities
that already exist in [vaultdb](../../../../../researches/vaultdb), a Rust
library/CLI for treating folders of Markdown-with-YAML-frontmatter as a
queryable database. The reinvention is the source of most of the data-layer
problems Eduport ships with today: parse-error handling, watcher-vs-writer
races, FTS5 reconcile, and the general fragility of glueing three language
runtimes (Rust shell + Python sidecar + SvelteKit frontend) over an HTTP
loopback.

This spec describes a rewrite of Eduport's data layer that:

- Replaces the Python sidecar with a Rust library `eduport-core` that depends
  on a library form of vaultdb (`vaultdb-core`).
- Promotes vaultdb itself from a binary-only crate into a Cargo workspace
  with a library crate, the existing CLI, and a new MCP server frontend.
- Treats the dual-structure nature of Markdown vaults — frontmatter (tabular)
  and wikilinks (graph) — as vaultdb's identity, with both first-class in
  the public API.
- Leaves Eduport's SvelteKit frontend mostly unchanged. Only the API client
  module is rewritten, swapping `fetch()` for Tauri `invoke()`.

The end state is a single Rust+SvelteKit Tauri application, no Python, no
HTTP loopback, no PyInstaller bundling, with a clean library boundary
between "Markdown as a database" (vaultdb-core) and "Eduport's domain layer"
(eduport-core).

## 2. Goals and non-goals

### Goals

- **Separate concerns at the library boundary.** "Markdown as a database" is
  one concern (vaultdb-core); "Eduport's typed schema, FTS5 search, file
  watcher, EML import, entity types" is another (eduport-core). Each is
  independently testable and reasonable to read.
- **Single source of truth for parse / write / link-graph semantics.** Today
  the Python sidecar and vaultdb implement these independently and will
  drift. After this rewrite, vaultdb-core owns them; eduport-core consumes.
- **Drop the Python sidecar entirely.** No Python in the build, no
  PyInstaller, no HTTP loopback, no port-discovery handshake.
- **Preserve all current Eduport user-visible features.** Same 8 entity
  types, same typed property schema, same saved-views model, same
  command-palette FTS5 search, same Obsidian-friendly storage layout.
- **Make vaultdb genuinely useful for projects beyond Eduport.** The
  rewrite ships vaultdb as a workspace with a library crate, a CLI crate,
  and an MCP server crate — three independent frontends from day one.
- **Design now for graph features later.** The query AST and link-graph
  type expose hooks so future graph features (shortest path, centrality,
  motif queries) become additive, not breaking.

### Non-goals

- **No frontend rewrite.** The SvelteKit codebase stays. Only
  `frontend/src/lib/api/*.ts` is touched, and only to swap HTTP `fetch` for
  Tauri `invoke`. No view changes, no component rewrites, no Svelte 5
  migration.
- **No model changes.** The 8 entity types, the 8 property types, the
  saved-views shape, the wikilink-as-foreign-key convention — all unchanged.
  This rewrite is about the data layer, not the data model.
- **No new graph features in this rewrite.** Graph-readiness hooks are
  designed; the features themselves (pathfinding, centrality, motifs) are
  deferred to a separate vaultdb effort.
- **No vaultdb-server or vaultdb-py crates yet.** The workspace is set up
  to accept them, but only `vaultdb-core`, `vaultdb-cli`, and `vaultdb-mcp`
  are built in this rewrite.
- **No Eduport functionality removed.** The Trash view, command palette,
  EML import, sample-data seeding — all preserved.

## 3. Architecture

Three layers, each with one job:

```
┌────────────────────────────────────────────────┐
│ eduport-app          (SvelteKit + Tauri shell) │
│   UI, routing, view state, components          │
└────────────────────────────────────────────────┘
                       │
┌────────────────────────────────────────────────┐
│ eduport-core         (Rust crate, eduport repo)│
│   typed schema · FTS5 · watcher · EML import   │
│   saved views · entity-type registry           │
└────────────────────────────────────────────────┘
                       │
┌────────────────────────────────────────────────┐
│ vaultdb-core         (Rust crate, vaultdb repo)│
│   parse · links · query · mutate · diagnose    │
└────────────────────────────────────────────────┘
                       │
              `.md` files on disk
```

The lower two boundaries are **library boundaries** (Rust crate
dependencies). The upper boundary (SvelteKit ↔ eduport-core) is a **Tauri
command boundary** — a Rust↔JS process boundary inside a single binary,
not a process boundary.

`vaultdb-cli` and `vaultdb-mcp` are sibling consumers of `vaultdb-core`
inside the vaultdb repo. They do not appear in Eduport's runtime; they
exist so vaultdb-core has multiple consumers from day one, which keeps its
public API honest.

### Stateful concerns live in eduport-core, not vaultdb-core

vaultdb's marketed thesis is **"no daemon, no cache, no state files — every
command reads files fresh."** That thesis is what makes vaultdb broadly
useful. A long-running file watcher and an FTS5 body-search index both
violate it. They therefore live in eduport-core, which is comfortable
holding mutable state because it backs a long-running GUI process.

Concretely:

- File watching: eduport-core uses the [`notify`](https://crates.io/crates/notify)
  crate (Rust equivalent of Python's `watchdog`) and emits typed
  `VaultEvent`s.
- Full-text search: eduport-core owns an SQLite database with FTS5 (same
  storage choice as today's sidecar) but built over vaultdb-core's parsed
  records, not its own parser.

When a vaultdb-driven mutation lands on disk, eduport-core's watcher sees
the file change and updates FTS5. There is no special path that bypasses
disk; both vaultdb-core mutations and external edits (from Obsidian, from
sync) are observed identically.

## 4. Repo and workspace shapes

### vaultdb (separate repo)

`/home/rusen/Desktop/codebase-shared/researches/vaultdb` becomes a Cargo
workspace:

```
researches/vaultdb/
├── Cargo.toml                workspace manifest
├── README.md                 dual-structure pitch (rewritten)
├── ARCHITECTURE.md           library scope discipline rule
└── crates/
    ├── vaultdb-core/         library: parse, links, query, mutate, schema
    ├── vaultdb-cli/          binary: today's CLI, slimmed to argparse + fmt
    └── vaultdb-mcp/          binary: MCP server exposing vault ops to LLMs
```

Future crates (`vaultdb-server`, `vaultdb-py`, `vaultdb-watch`) get added
to this workspace when a real second consumer for each appears. The
workspace is structured to accept them without restructuring.

### eduport (this repo)

`/home/rusen/Desktop/codebase-shared/rusen/eduport` becomes a Cargo
workspace:

```
rusen/eduport/
├── Cargo.toml                workspace manifest
├── crates/
│   ├── eduport-core/         library: schema, FTS5, watcher, EML, view store
│   └── eduport-tauri/        binary: thin Tauri shell, calls into core
├── frontend/                 SvelteKit, mostly unchanged
└── (sidecar/ deleted at end of phase 11)
```

`eduport-core/Cargo.toml` declares vaultdb-core as a dependency:

```toml
vaultdb-core = { path = "../../../../researches/vaultdb/crates/vaultdb-core" }
```

Path dependency is fine for active co-development. Once vaultdb publishes
tagged versions, this becomes a git ref or a registry version.

`eduport-tauri` replaces today's `src-tauri` directory and keeps the same
build/install pipeline (Tauri 2, the `package.json` scripts that drive
`tauri build`, the icon set, etc.).

## 5. vaultdb-core public API

This is the contract the library exposes. Internals are free to evolve.

### Top-level surface

```rust
// crates/vaultdb-core/src/lib.rs

pub mod vault;       // Vault, discovery, file enumeration
pub mod record;      // Record, Value, virtual fields
pub mod query;       // Expr (the AST), Predicate, Query
pub mod links;       // LinkGraph, link extraction
pub mod mutation;    // UpdateBuilder, MutationReport, Rename, Delete, Move
pub mod schema;      // generic schema (infer/validate)
pub mod error;       // VaultdbError, ParseError

pub use vault::Vault;
pub use record::{Record, Value};
pub use query::{Expr, Predicate, Query};
pub use links::LinkGraph;
pub use mutation::{
    DeleteBuilder, MoveBuilder, MutationReport, RenameBuilder, UpdateBuilder,
};
pub use error::{VaultdbError, ParseError};
```

### `Vault` is the entry point

```rust
pub struct Vault {
    pub root: PathBuf,
}

impl Vault {
    pub fn discover(start: &Path) -> Result<Self, VaultdbError>;
    pub fn with_root(root: PathBuf) -> Self;

    /// Load records with parse diagnostics.
    /// The application layer decides whether to surface, log, or ignore
    /// parse errors — vaultdb does not silently drop them.
    pub fn load_records(
        &self,
        folder: &str,
        recursive: bool,
    ) -> Result<LoadResult, VaultdbError>;

    /// Single-record lookup by name (no folder scan when caller knows
    /// the name).
    pub fn find_by_name(
        &self,
        folder: &str,
        name: &str,
    ) -> Result<Option<Record>, VaultdbError>;

    /// Run a structured query.
    pub fn query(&self, q: &Query) -> Result<Vec<Record>, VaultdbError>;

    /// Build the link graph for a scope.
    pub fn link_graph(
        &self,
        scope: GraphScope,
    ) -> Result<LinkGraph, VaultdbError>;
}

pub struct LoadResult {
    pub records: Vec<Record>,
    pub parse_errors: Vec<ParseError>,
}

pub struct ParseError {
    pub file: PathBuf,
    pub message: String,
}
```

### `Value` is vaultdb's own enum

vaultdb defines its own value type rather than re-exporting `serde_yaml::Value`,
following the convention of every comparable library (`toml::Value`,
`serde_json::Value`). This insulates the public API from `serde_yaml` itself,
which is currently unmaintained.

```rust
pub enum Value {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    List(Vec<Value>),
    Map(BTreeMap<String, Value>),
}

impl Value {
    pub fn as_string(&self) -> Option<&str>;
    pub fn as_list(&self) -> Option<&[Value]>;
    pub fn is_null(&self) -> bool;
    // ... convenience accessors
}

impl From<serde_yaml::Value> for Value { /* ... */ }
impl From<Value> for serde_yaml::Value { /* ... */ }
```

### Query AST is `pub` and treats links as first-class

This is the dual-structure thesis enforced at the type level. Frontmatter
predicates and link predicates are siblings in the same enum, not a
"main feature" and "extra flag".

```rust
pub enum Expr {
    Predicate(Predicate),
    And(Vec<Expr>),
    Or(Vec<Expr>),
    Not(Box<Expr>),
    LinksTo(LinkPredicate),
    LinkedFrom(LinkPredicate),
}

pub enum Predicate {
    Equals { field: String, value: Value },
    Contains { field: String, value: Value },
    Compare { field: String, op: CompareOp, value: Value },
    Matches { field: String, regex: String },
    Exists { field: String },
}

pub enum CompareOp { Lt, Le, Gt, Ge, Ne }

pub enum LinkPredicate {
    Target(String),         // links-to "React"
    Where(Box<Expr>),       // links-to-where (tags contains topic/ai)
}

impl FromStr for Expr {
    type Err = VaultdbError;
    fn from_str(s: &str) -> Result<Self, Self::Err>;
}

impl Expr {
    /// Convenience wrapper over FromStr; the CLI's --where flag uses this.
    pub fn parse(s: &str) -> Result<Self, VaultdbError> {
        s.parse()
    }
}
```

eduport-core builds `Expr` directly from typed-schema knowledge (e.g.,
"this field is a single-select with options [reach, target, safety]");
the CLI uses `Expr::parse` to compile its `--where` flag. Both produce
the same AST, walked by the same query engine.

### `Query` bundles filter + projection + sort + limit

```rust
pub struct Query {
    pub folder: String,
    pub filter: Option<Expr>,
    pub select: Option<Vec<String>>,   // None = all fields
    pub sort: Option<SortKey>,
    pub limit: Option<usize>,
    pub recursive: bool,
}

pub struct SortKey {
    pub field: String,
    pub descending: bool,
}
```

### Mutations are typed, builder-pattern, and dry-run-by-construction

```rust
pub struct UpdateBuilder { /* private fields */ }

impl UpdateBuilder {
    pub fn new(filter: Expr) -> Self;
    pub fn set(self, field: &str, value: Value) -> Self;
    pub fn unset(self, field: &str) -> Self;
    pub fn add_tag(self, tag: &str) -> Self;
    pub fn remove_tag(self, tag: &str) -> Self;

    /// Returns a plan; nothing has touched disk yet.
    pub fn plan(&self, vault: &Vault)
        -> Result<MutationReport, VaultdbError>;

    /// Plans, then writes the planned changes to disk.
    pub fn execute(self, vault: &Vault)
        -> Result<MutationReport, VaultdbError>;
}

pub struct MutationReport {
    pub changes: Vec<PlannedChange>,
    pub errors: Vec<MutationError>,
}

pub struct PlannedChange {
    pub path: PathBuf,
    pub before: BTreeMap<String, Value>,
    pub after: BTreeMap<String, Value>,
}
```

`plan()` and `execute()` being separate is the dry-run model lifted into
the library. The CLI's `--dry-run` is implemented as `plan() + render`,
not a special code path. `vaultdb-mcp` exposes `plan()`-only tools to
LLM agents by default — agents propose changes, humans approve, then
the application calls `execute()`.

Equivalent builders exist for `Delete`, `Move`, and `Rename`:

```rust
pub struct DeleteBuilder { /* ... */ }
pub struct MoveBuilder { /* ... */ }
pub struct RenameBuilder { /* ... */ }
```

`RenameBuilder` is the most domain-aware: it auto-rewrites all
`[[wikilinks]]` across the vault that point at the renamed file. This is
existing vaultdb behaviour, just wrapped in the typed API.

### Serialization

All public data types implement `serde::Serialize` and `serde::Deserialize`.
This makes them usable across IPC boundaries (Tauri commands,
`vaultdb-mcp` JSON-RPC, future HTTP) for free.

### Link graph

```rust
pub struct LinkGraph { /* private fields */ }

pub enum GraphScope {
    All,
    Folder(String),
    Where(Expr),
}

impl LinkGraph {
    pub fn outgoing(&self, name: &str) -> &[String];
    pub fn incoming(&self, name: &str) -> &[String];
    pub fn unresolved(&self) -> Vec<UnresolvedLink>;
    pub fn traverse_from(
        &self,
        name: &str,
        depth: usize,
        direction: Direction,
    ) -> Vec<String>;
}

pub enum Direction { Outgoing, Incoming, Both }
```

Future graph features (shortest path, centrality, motifs) become inherent
methods on `LinkGraph` — purely additive, no breaking changes.

### Errors

```rust
#[derive(Debug, thiserror::Error)]
pub enum VaultdbError {
    #[error("vault not found from {0}")]
    VaultNotFound(String),
    #[error("folder not found: {0}")]
    FolderNotFound(String),
    #[error("invalid query: {0}")]
    InvalidQuery(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    // ... etc
}
```

`anyhow::Error` is allowed inside `vaultdb-cli` (a binary) but not exposed
from `vaultdb-core` (a library).

## 6. eduport-core surface

```
crates/eduport-core/src/
├── lib.rs
├── error.rs                EduportError
├── settings.rs             data folder, persistence
├── entity_type.rs          the 8 entity types as a const registry
├── schema/
│   ├── mod.rs
│   ├── property_type.rs    8 typed property types
│   ├── store.rs            .eduport/schema.yaml
│   └── validator.rs        validates a Record against the typed schema
├── view/
│   ├── mod.rs
│   └── store.rs            .eduport/views.yaml
├── search/
│   ├── mod.rs              SQLite/FTS5 wrapper
│   ├── reconcile.rs        rebuild from vault state on startup
│   └── index.rs            upsert/delete on watcher events
├── watcher.rs              notify-based, emits typed VaultEvent
├── eml.rs                  .eml → Email entity markdown
└── tauri/
    ├── mod.rs
    └── commands/
        ├── entity.rs
        ├── search.rs
        ├── schema.rs
        ├── view.rs
        ├── trash.rs
        └── settings.rs
```

### Two surfaces: pure-Rust API and Tauri command layer

`eduport-core` exposes:

1. A pure-Rust API on an `Eduport` struct (no Tauri dependency). Testable
   from `#[test]` without Tauri's runtime.
2. A thin Tauri command layer (`tauri/commands/`) that holds `Eduport` as
   Tauri-managed state and wraps the pure API as `#[tauri::command]`s.

This split looks like duplication but pays off in two ways: tests don't
need Tauri's runtime, and a future CLI for Eduport (scripting the data
layer outside the GUI) is free — call the same pure API from a small bin.

### Tauri command shape (simplified)

```rust
#[tauri::command]
async fn list_entities(
    state: State<'_, Eduport>,
    kind: EntityKind,
    filter: Option<EntityFilter>,
) -> Result<Vec<EntitySummary>, EduportError>;

#[tauri::command]
async fn get_entity(
    state: State<'_, Eduport>,
    id: String,
) -> Result<Entity, EduportError>;

#[tauri::command]
async fn create_entity(
    state: State<'_, Eduport>,
    kind: EntityKind,
    fields: BTreeMap<String, Value>,
) -> Result<Entity, EduportError>;

#[tauri::command]
async fn update_entity(
    state: State<'_, Eduport>,
    id: String,
    changes: EntityChanges,
) -> Result<Entity, EduportError>;

#[tauri::command]
async fn delete_entity(
    state: State<'_, Eduport>,
    id: String,
) -> Result<(), EduportError>;

#[tauri::command]
async fn search(
    state: State<'_, Eduport>,
    query: String,
) -> Result<Vec<SearchHit>, EduportError>;
```

Plus equivalents for schema CRUD, view CRUD, trash list/restore/empty,
EML import, settings read/write.

### Watcher events flow as Tauri events, not commands

`eduport-core::watcher` emits typed `VaultEvent`s. The Tauri layer
forwards them via Tauri's event channel:

- `entity-changed` `{ id, kind }`
- `entity-deleted` `{ id }`
- `schema-changed`
- `views-changed`
- `parse-error` `{ path, message }`

Frontend subscribes once at startup; each event mutates the relevant
Svelte store; components reactively re-render. This is push-based; no
polling, no manual re-fetch after mutations.

### Frontend changes

`frontend/src/lib/api/*.ts` keeps every existing function signature. The
implementation underneath swaps `fetch(url)` for `invoke(command, args)`.
Svelte stores, components, view code, routing — all untouched.

A "stop-and-flag" rule applies: any signature change in `api/*.ts`
during this rewrite requires explicit justification. If we find ourselves
needing one, that's a sign the swap is doing more than the brief said,
and we should pause and decide whether the change belongs in this rewrite
or a follow-up.

## 7. Library scope discipline

To be saved as `researches/vaultdb/ARCHITECTURE.md`:

> **Library scope discipline.** Every change to vaultdb-core must serve at
> least one consumer that is not Eduport. If you cannot describe a change
> without mentioning Eduport, the change belongs in eduport-core.
>
> Exception: parse-error and link-resolution edge cases discovered while
> building Eduport are bug fixes, not features. They apply to all
> consumers and stay in vaultdb.
>
> **Acceptance gate for vaultdb features**: imagine a user of `vaultdb-mcp`
> (an LLM agent) or a future Markdown-tool author. Would they want this
> change? If the answer requires explaining Eduport's domain, the change
> is in the wrong layer.

This rule keeps vaultdb-core a generic library and stops the rewrite from
silently turning vaultdb into "the eduport sidecar in Rust".

## 8. Graph-features-later: API hooks

Direction (3) — first-class graph features (shortest path, centrality,
motif queries) — is **not implemented in this rewrite**. The shapes drawn
in section 5 admit (3) without breaking changes:

- `Expr::LinksTo(LinkPredicate)` and `Expr::LinkedFrom(LinkPredicate)` are
  first-class AST variants. Future graph predicates (`OnPathFrom`,
  `WithinDistance`, `CentralityAbove`) become new variants in the same
  enum — purely additive.
- `LinkGraph` is a public type returned by `Vault::link_graph(scope)`.
  Today's methods cover direct neighbours and BFS traversal; tomorrow
  gains `shortest_path`, `pagerank`, `betweenness`, and motif queries
  as inherent methods. No type changes.
- `Vault::link_graph(scope: GraphScope)` accepts subset queries
  (`Folder`, `Where(Expr)`, `All`) so future graph queries can constrain
  the working set without re-walking the vault.

The point of these hooks is to prevent (3) from being a refactor when we
get there. None of them adds behaviour today.

## 9. Migration order

Eleven phases. Each is independently shippable and verifiable; no phase
depends on a future phase being half-built.

1. **vaultdb workspace split.** Refactor today's binary into a workspace:
   `vaultdb-core` (library only) + `vaultdb-cli` (binary using core). No
   behaviour change. Today's CLI test suite passes unchanged.

2. **vaultdb-core API hardening.** Implement section 5: `LoadResult` with
   parse diagnostics, `find_by_name`, public `Expr` AST, `UpdateBuilder`
   and siblings, `LinkGraph`, `Value` enum, `Serialize`/`Deserialize`
   everywhere. CLI re-implemented over the new core; tests still pass.

3. **vaultdb-mcp.** First MCP-frontend release. Exposes `query`, `links`,
   `traverse`, `unresolved`, plus *plan-only* mutation tools (no
   `execute()` exposed yet). Validates the library API isn't accidentally
   CLI-shaped.

4. **eduport repo workspace conversion.** Turn `rusen/eduport` into a
   Cargo workspace; scaffold `crates/eduport-core`; move existing
   `src-tauri/` to `crates/eduport-tauri/`. No behaviour change.

5. **eduport-core scaffold + Settings/SchemaStore/ViewStore ports.** Port
   the smallest, most isolated pieces of the Python sidecar first. Minimal
   vaultdb-core dependency at this stage; this is the warm-up.

6. **eduport-core entity CRUD.** Full entity read/write via vaultdb-core.
   EML parser ported. Meatiest phase.

7. **eduport-core FTS5.** Port the SQLite/FTS5 layer from the Python
   sidecar, but rewritten to consume vaultdb-core's parsed records (no
   second parser).

8. **eduport-core watcher.** `notify`-based, emits typed `VaultEvent`;
   FTS5 listens and updates.

9. **eduport-tauri command wiring.** Tauri commands forward to
   `eduport-core`; watcher events forwarded as Tauri events.

10. **Frontend API client swap.** `frontend/src/lib/api/*.ts` swaps
    `fetch` → `invoke`. Function signatures unchanged.

11. **End-to-end verify, then delete `sidecar/`.** Cut over only after a
    feature-parity check across all 8 entity types and all view modes.
    There is no intermediate state where some entity types use the old
    sidecar and some use the new core — the cutover is one switch.

Each phase gets its own implementation plan written when the prior phase
has shipped. Trying to plan phase 7 now would lock in decisions we don't
yet have evidence to make.

## 10. Open questions and risks

- **`vaultdb-mcp` tool surface.** The exact set of MCP tools and their
  argument shapes is deferred to phase 3's plan. The principle is
  plan-only mutation tools; the surface is TBD.
- **vaultdb publishing cadence.** Path dependency works fine for active
  co-development. The decision of when to start publishing tagged
  versions of `vaultdb-core` (and switch eduport to a version dep) is
  deferred until vaultdb-core's API has stabilised post-phase-2.
- **External edits to `.eduport/schema.yaml` that break existing entities.**
  Today's sidecar handles "the user edited schema.yaml in Obsidian and now
  some entity rows fail validation" by recording per-row parse errors in
  SQLite. eduport-core preserves this behaviour: the schema validator is
  permissive (records load even if invalid; validation results are
  surfaced via a `parse-error` Tauri event). Documented here so it's not
  silently dropped.
- **vaultdb's `serde_yaml` dependency.** `serde_yaml` is currently
  unmaintained; community is migrating to `serde_yml` (a fork). The
  `Value` abstraction (section 5) insulates consumers from this; the
  internal switch happens whenever vaultdb decides, not on
  eduport-core's clock. Documented so the abstraction's purpose is
  explicit.
- **Watcher debouncing.** `notify` raises events at the file-system rate.
  eduport-core needs a debounce to avoid thrashing FTS5 during sync
  storms (Dropbox / iCloud / Syncthing can deliver hundreds of events
  per second on initial sync). Debounce policy (window length, coalesce
  rules) is deferred to phase 8's plan.
- **Tauri command async runtime.** Tauri 2 requires `async fn` for
  commands but eduport-core's pure API is synchronous (file IO is fast
  enough at typical vault scale). The Tauri layer wraps sync calls in
  `tokio::task::spawn_blocking` where appropriate. Confirmed in phase 9.

## 11. Out-of-scope follow-ups

These are real opportunities surfaced during brainstorming but
deliberately left out of this rewrite. Each is its own future spec.

- **Direction (3): graph features.** Pathfinding, centrality, bidirectional
  relations, motif queries. Hooks designed in this spec; features in a
  separate effort.
- **`vaultdb-server` (HTTP frontend).** A generalisation of today's
  Python sidecar. Useful when a future markdown-vault app wants
  vaultdb-as-a-service.
- **`vaultdb-py` (Python bindings via pyo3).** Useful for Jupyter /
  data-science workflows over a vault.
- **`vaultdb-watch` (separate crate).** Lifts eduport-core's watcher
  logic into a generic vaultdb-aware change-event library, once a second
  consumer materialises.
- **CLI for eduport-core.** Free once eduport-core's pure API exists;
  built when there's a real script-the-vault use case.
- **Frontend modernisation.** The SvelteKit frontend has accumulated
  sidecar-driven complexity (sync state, parse-error surfacing,
  watcher-event handling) that may simplify once eduport-core is in
  place. Out of scope here; revisit after phase 11 ships.
