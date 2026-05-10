# Phase 3: vaultdb-mcp Implementation Plan

**Goal:** Add a third workspace member to vaultdb — `vaultdb-mcp` — that
exposes the vaultdb-core public API as a Model Context Protocol server over
stdio, so LLM agents (Claude Desktop, Cursor, generic MCP clients) can query
and reason about a vault.

**Architecture:** Thin frontend over `vaultdb-core`. One binary, stdio
transport, `rmcp` 1.6 SDK for protocol plumbing. Each tool maps 1:1 to a
core API call. **Mutations are plan-only by default** — agents propose,
humans approve, then a host calls `execute()` itself (which goes through
the CLI or the Tauri layer, not through MCP).

**Why this matters for the spec:** vaultdb-core stays honest if it has
multiple frontends. Up to now it has had one (the CLI). Adding a second
forces every public API decision to be evaluated against "would this make
sense in a non-CLI context?" That is the library scope discipline rule
from spec §7, made operational.

## Tool surface (read-only)

| Tool             | vaultdb-core call                  | Purpose                                                |
|------------------|------------------------------------|--------------------------------------------------------|
| `query`          | `Vault::query(&Query)`             | Run a structured query, return matching records.       |
| `find_by_name`   | `Vault::find_by_name(folder,name)` | One-shot record lookup.                                |
| `list_folders`   | walk `Vault::root` for `.md` dirs  | Discover what folders the vault has.                   |
| `links`          | `LinkGraph::{outgoing,incoming}_links` | Direct neighbors of a note.                        |
| `traverse`       | `LinkGraph::traverse_from`         | BFS through the graph from a starting note.           |
| `unresolved`     | `LinkGraph::unresolved`            | Dangling wikilinks across the vault.                  |
| `schema_show`    | `schema::load_schema`              | Show the typed schema (if `vaultdb-schema.yaml` exists). |
| `schema_infer`   | `schema::infer_schema`             | Generate a schema from existing data, return as YAML. |

## Tool surface (plan-only mutations)

| Tool             | vaultdb-core call                                  | Purpose                                       |
|------------------|----------------------------------------------------|-----------------------------------------------|
| `plan_update`    | `UpdateBuilder::plan`                              | Show what an update would change.            |
| `plan_delete`    | `DeleteBuilder::plan`                              | Show which files would be moved to .trash.   |
| `plan_move`      | `MoveBuilder::plan`                                | Show which files would relocate.             |
| `plan_rename`    | `RenameBuilder::plan`                              | Show the rename + every backlink rewrite.    |

There are intentionally no `execute_*` tools. Agents can describe a change
they'd like to make, and the host renders the plan to a human; the host
itself runs the `execute()` if the human approves. This matches the spec's
"plan-only mutation tools" requirement (§5, §9 phase 3).

## Vault-root resolution

The MCP server is launched as a child process (typically by Claude Desktop
or similar). It needs to know which vault to open. Two mechanisms:

1. `--vault <path>` CLI flag (highest priority).
2. `VAULTDB_VAULT` env var.
3. Auto-discovery from cwd via `Vault::discover` (lowest priority).

If none of these resolve to a vault, the server starts but every tool call
returns an `InvalidParams` error explaining the resolution failure rather
than crashing. This is friendlier than refusing to start.

## File structure

```
crates/vaultdb-mcp/
├── Cargo.toml
└── src/
    ├── main.rs          binary entry: arg parsing, vault resolution, server bootstrap
    ├── server.rs        Server struct, holds Vault, dispatches tool calls
    ├── params.rs        JsonSchema-deriving param structs for each tool
    └── tools/
        ├── mod.rs       pub mod query; pub mod links; pub mod mutations; pub mod schema;
        ├── query.rs     query, find_by_name, list_folders
        ├── links.rs     links, traverse, unresolved
        ├── mutations.rs plan_update, plan_delete, plan_move, plan_rename
        └── schema.rs    schema_show, schema_infer
```

Per spec §6 (file structure principle): one responsibility per file, files
that change together live together, prefer focused over layered.

## Tasks

### Task 1: Add vaultdb-mcp crate to the workspace

**Files:**
- Modify: `Cargo.toml` (workspace root) — add `crates/vaultdb-mcp` to `members`
- Create: `crates/vaultdb-mcp/Cargo.toml`
- Create: `crates/vaultdb-mcp/src/main.rs` (placeholder hello-world that proves wiring)

- [ ] **Step 1: Add the member to the workspace manifest**

```toml
[workspace]
resolver = "2"
members = [
    "crates/vaultdb-core",
    "crates/vaultdb",
    "crates/vaultdb-mcp",
]
```

- [ ] **Step 2: Write the crate manifest** with `rmcp`, `tokio`, `vaultdb-core`, `clap`, `anyhow`, `schemars`, `serde`, `serde_json` dependencies, version/edition/license/repository inherited from workspace.

- [ ] **Step 3: Write a minimal `main.rs`** that just prints a vaultdb-mcp banner and exits 0, just to prove the crate compiles in the workspace.

- [ ] **Step 4: Run `cargo build -p vaultdb-mcp`**. Expected: compiles.

- [ ] **Step 5: Commit.**

### Task 2: Vault resolution helper

**Files:**
- Create: `crates/vaultdb-mcp/src/main.rs` — replace placeholder with real arg parsing
- Create: `crates/vaultdb-mcp/src/server.rs` — `VaultdbServer` skeleton

- [ ] **Step 1: Write the vault-resolution function in main.rs** with the three-tier fallback (`--vault`, `VAULTDB_VAULT`, `Vault::discover`). Returns `Option<Vault>`.

- [ ] **Step 2: Define `VaultdbServer { vault: Option<Vault> }` in server.rs** — the `Option` is so the server starts even when no vault is found, and tool calls return a typed error.

- [ ] **Step 3: Add a `vault()` accessor on `VaultdbServer`** that returns `Result<&Vault, McpError>` with a clear "no vault found" message for tool implementations to propagate.

- [ ] **Step 4: Run `cargo build`.**

- [ ] **Step 5: Commit.**

### Task 3: Wire stdio MCP server with rmcp's tool_router

**Files:**
- Modify: `crates/vaultdb-mcp/src/server.rs` — add `#[tool_router]` impl with one trivial tool (`ping`) returning `"pong"`
- Modify: `crates/vaultdb-mcp/src/main.rs` — call `VaultdbServer::serve(stdio()).await`

- [ ] **Step 1: Add `#[tool_router(server_handler)] impl VaultdbServer { ... }`** with a single `#[tool] ping(&self) -> String` returning `"pong"`.

- [ ] **Step 2: In main.rs, construct the server and call `.serve(stdio()).await`.** Use `#[tokio::main]` with the multi-thread runtime.

- [ ] **Step 3: Run `cargo build`** and verify no errors.

- [ ] **Step 4: Add an integration test** that spawns the binary and sends one `tools/list` JSON-RPC over stdio, expecting `ping` to appear. (If this is too complex for the initial drop, fall back to a "binary runs and exits cleanly" smoke test and defer the JSON-RPC harness to a later commit.)

- [ ] **Step 5: Commit.**

### Task 4: Read tools — query, find_by_name, list_folders

**Files:**
- Create: `crates/vaultdb-mcp/src/params.rs`
- Create: `crates/vaultdb-mcp/src/tools/mod.rs`
- Create: `crates/vaultdb-mcp/src/tools/query.rs`

- [ ] **Step 1: In params.rs, define the param structs** for the read tools, each deriving `serde::Deserialize` and `schemars::JsonSchema`. Example: `QueryParams { folder, where_clause, select, sort, limit, recursive }`.

- [ ] **Step 2: In tools/query.rs, write `query`, `find_by_name`, `list_folders` tool functions** as standalone async functions taking `&VaultdbServer` and the params. Each parses params into the corresponding vaultdb-core call, runs it, and returns the result serialized as JSON. Errors map to `McpError` with descriptive messages.

- [ ] **Step 3: In server.rs, add `#[tool]` wrappers** in the `#[tool_router]` impl that delegate to the standalone functions. This keeps server.rs a thin router and tools/*.rs the real implementations.

- [ ] **Step 4: Run cargo build + cargo test.** Expected: clean.

- [ ] **Step 5: Commit.**

### Task 5: Read tools — links, traverse, unresolved

**Files:**
- Create: `crates/vaultdb-mcp/src/tools/links.rs`
- Modify: `crates/vaultdb-mcp/src/server.rs` (add tool router entries)
- Modify: `crates/vaultdb-mcp/src/tools/mod.rs` (export `links`)

Same structure as Task 4: param structs in params.rs, implementations in
tools/links.rs, thin wrappers in server.rs. Tools delegate to
`Vault::link_graph`, `LinkGraph::traverse_from`, `LinkGraph::unresolved`.

### Task 6: Schema tools — schema_show, schema_infer

**Files:**
- Create: `crates/vaultdb-mcp/src/tools/schema.rs`
- Modify: `crates/vaultdb-mcp/src/server.rs`

`schema_show` reads `<vault>/vaultdb-schema.yaml` and returns the parsed
schema as JSON; `schema_infer` walks a folder and returns
`schema::infer_schema(folder, &records)` rendered to YAML.

### Task 7: Plan-only mutation tools

**Files:**
- Create: `crates/vaultdb-mcp/src/tools/mutations.rs`
- Modify: `crates/vaultdb-mcp/src/server.rs`

`plan_update`, `plan_delete`, `plan_move`, `plan_rename` — each parses an
`Expr` from a `where` string (`Expr::parse`), constructs the corresponding
builder, calls `.plan(&vault)`, and returns the `MutationReport` as JSON.
**No `execute` variants.** Add a doc note on each tool describing this
restriction.

### Task 8: Top-level tests, README, smoke run

- [ ] **Step 1: Add `crates/vaultdb-mcp/tests/smoke.rs`** that boots a vault in a TempDir, spawns the binary, sends a couple of MCP tool calls over stdio, and asserts the responses. (Use a small helper to do JSON-RPC framing.)

- [ ] **Step 2: Update `README.md` (workspace-level)** with a "vaultdb-mcp" subsection: how to launch, how Claude Desktop is configured to point at the binary, the `--vault` flag, what tool calls are supported.

- [ ] **Step 3: Run the full workspace test suite + clippy + fmt --check.**

- [ ] **Step 4: Final commit** + update memory file with Phase 3 SHA.
