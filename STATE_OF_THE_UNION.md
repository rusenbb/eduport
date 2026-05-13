# vaultdb + eduport — State of the Union

**Date:** 2026-05-11
**Author:** Claude (audit + recommendation)
**Audience:** rusenbb, project lead

> **Status update — 2026-05-13:** much of this doc is now historical.
> Below is the body as originally written; the items it called out have
> partially shipped. Skim the status table before treating any
> specific recommendation as live.

---

## Status update — 2026-05-13 (what shipped since this doc)

| TL;DR item | Status |
|---|---|
| Library-scope discipline (vaultdb-core ↔ eduport-core boundary) | ✅ Held. Eduport-core no longer reimplements queries; the ORM-substrate migration landed cleanly. |
| 1. Second consumer | ✅ eduport-core moved onto the typed-ORM substrate with `#[derive(vaultdb_orm::Note)]` for all 8 entity types, plus a `schema::vaultdb_bridge` that converts eduport's Notion-style schema into a runtime vaultdb `CollectionSchema`. See vaultdb 1.1.0 CHANGELOG and eduport 0.2.0. |
| 2. Benchmarks at scale | ⚠️ Partial. `BENCHMARKS.md` exists with 1k / 10k / 100k numbers, but no continuous-benchmark regression CI yet. |
| 3. Stability contract | ⚠️ Pre-1.x ARCHITECTURE.md mentions semver intent but there's no `STABILITY.md` consumers can pin to. Still open. |
| 4. FTS in the library | ✅ `vaultdb-fts` is an opt-in companion crate consuming `vaultdb-core::Record`. Not in-core (deliberate — keeps core stateless), but no longer "every consumer rolls their own." |
| 5. Migration / upgrade story | ⚠️ Open. No `MIGRATIONS.md` yet. 1.0 → 1.1 was additive so no migration needed in practice; the gap matters when 2.0 lands. |

Additional shipped since 05-11 (not in original TL;DR):

- Schema-aware create end-to-end: `CreateBuilder` in `vaultdb-core`, `plan_create` MCP tool, `execute_*` MCP tools gated by `--dangerously-allow-*` flags, `Create<T>` in vaultdb-orm. All three frontends share one create path.
- Three new schema field types: `wikilink`, `date`, `url`.
- Schema defaults (`default:` literal and `default_expr:` for `today` / `now` / `epoch`).
- `#[derive(Note)]` gained `collection = "..."` so `Create<T>` auto-resolves the matching YAML collection. `filter` became `discriminator`; `folder = ""` workaround dropped for tag-discriminated entities.
- Bool literal coercion in CLI / DSL string paths (1.1.1).

The audit's "everything else is maintenance" framing still applies: the open items are real but not blocking, and the substrate migration was the right call.

---

## TL;DR

You are building the right thing in roughly the right way. The
**vaultdb-core ↔ eduport-core** boundary is *clean today* — after the
substrate migration we just merged, eduport no longer reimplements
queries that vaultdb already does. That single piece of discipline is
the most important asset in this whole project. Most "library + first
app" splits fail right here.

But the project is **not yet production-grade**, and the phrase
"SQLite for markdown" is doing more work than the code currently
backs up. The five things missing between "credible library" and
"production-grade professional thing" are:

1. **A second consumer.** Until something other than eduport depends
   on vaultdb, the API will silently warp around eduport's needs.
2. **Benchmarks at scale.** "Scales well" is unproven past ~1k records.
   SQLite's reputation comes from decades of measured behaviour at
   millions of rows; vaultdb has zero published numbers.
3. **A stability contract.** v1.0.0 was published, but there is no
   written semver-versus-breaking-change policy that downstream users
   can read and trust.
4. **Full-text search in the library.** Every serious consumer needs
   FTS. Forcing each one to maintain its own SQLite/FTS5 sidecar
   (like eduport does) is a tax that will deter adoption.
5. **A migration / upgrade story.** When vaultdb v2 ships with
   breaking changes, what does a v1 user do? There's no doc for that.

Everything else (codebase cleanup, UX polish, CI hardening) is
**maintenance, not strategy.** Important, but not what determines
whether this "takes off." This report covers both layers.

---

## 1. Where we are today

### 1.1 vaultdb (the engine)

**Repo:** `/home/rusen/Desktop/codebase-shared/researches/vaultdb`
**Version:** v1.0.0 (shipped to crates.io, PyPI, npm)

| Crate | LoC | Purpose |
|---|---:|---|
| `vaultdb-core` | 7,669 | Library: Vault, Query, Expr, mutations, LinkGraph, where-DSL |
| `vaultdb` | 2,039 | CLI binary (10 subcommands) |
| `vaultdb-mcp` | 782 | MCP server (plan-only mutations for LLM agents) |
| `bindings/vaultdb-pyo3` | 271 | Python eager-query bindings |
| `bindings/vaultdb-wasm` | 202 | Browser parser-only bindings |

**Public API contract** (pinned at `crates/vaultdb-core/src/lib.rs`):

- **Read:** `Vault::query()` (eager), `Vault::query_iter()`
  (streaming, O(1) RAM in the pure-stream case)
- **Write:** `UpdateBuilder` / `DeleteBuilder` / `MoveBuilder` /
  `RenameBuilder` — each has `.plan(&vault)` (read-only preview) and
  `.execute(&vault)` (atomic disk write)
- **Graph:** `LinkGraph::build(records)` rebuilds on demand; no
  cache between calls
- **Where-DSL:** pest-driven grammar at `where_dsl.pest`. Operators
  `= < > <= >= != contains startswith endswith matches IN IS NULL`
  plus `&& || NOT ( )` and `_body / _name / _path / _modified /
  _links / _backlinks` virtual fields

**Quality signals:**

- 182 `#[test]` annotations across 15 files
- CI: `cargo test --workspace`, `cargo clippy -D warnings`,
  `cargo fmt --check`, and `cargo publish --dry-run` for the
  library (catches manifest drift)
- Released to crates.io, PyPI, npm
- Documented: `ARCHITECTURE.md` (scope rules), `README.md`,
  `RELEASE.md`, `CHANGELOG.md`, `docs/SAFETY.md`
- **Zero TODO/FIXME/XXX comments. Zero `unimplemented!()` or
  `todo!()`.** This is unusually disciplined.

**Architectural promises (`ARCHITECTURE.md`):**

> "No daemon, no cache, no state files. Every read traverses the
> filesystem fresh."

This is the **library's defining choice.** Consumers (like eduport)
who need caching, FTS, or watchers add those *themselves*. vaultdb
stays small and predictable.

### 1.2 eduport (the first app)

**Repo:** `/home/rusen/Desktop/codebase-shared/rusen/eduport`
**Version:** v0.1.1 (Tauri + SvelteKit 5 desktop app)

| Crate / dir | LoC | Purpose |
|---|---:|---|
| `eduport-core` | 6,301 | Domain layer: entity types, schema, FTS5, watcher, EML import |
| `eduport-tauri` | 2,736 | Tauri shell (36 commands) |
| `frontend/` | 8,207 | SvelteKit 5 UI (11 routes) |

**What eduport-core adds *over* vaultdb-core:**

1. **Typed entity model** — 8 frozen types (University, Lab, Person,
   Program, Application, Document, Email, Note) discriminated by
   the `eduport-type/<value>` tag in the frontmatter `tags:` list.
   Files live flat at the vault root — no per-type folders.
2. **Custom property schema** — user-defined fields (text, number,
   date, checkbox, select, relation) stored in a `.eduport/schema.yaml`,
   with strict shape validation and historical constraints
   (no option-value renames, no type changes post-creation).
3. **FTS5 search index** at `.eduport/index.sqlite` —
   mtime-keyed incremental updates, full-text search on body +
   name + tags + custom text fields. *This is the one thing not in
   vaultdb yet.*
4. **File watcher** (`notify-debouncer-full`) emitting typed
   `VaultEvent` enums to the frontend.
5. **EML import** (drag-drop email files → Email entities).
6. **Saved views** (per-type filter/sort/group configurations
   persisted to disk).
7. **Parse-error UX surface** (the Status page shows files with
   malformed frontmatter).

**Quality signals (post-merge):**

- 122 `#[test]` annotations in eduport-core
- 0 svelte-check errors / warnings (455 files)
- 0 TODO/FIXME/XXX comments
- 0 `console.log` statements
- Lint clean under `-D warnings`
- Comprehensive `HANDOVER.md` (192 LoC) documenting conventions,
  pending verification, and gotchas

**Recent merges (today):**

- PR #2: 11 commits of UX polish (shortcuts, toasts, context menus,
  dashboard, deadlines, command-palette footer, skeletons)
- PR #3: the **substrate refactor** — filter/list/aggregate now
  route through `Vault::query()`. The `properties` and `entity_tags`
  shadow SQLite tables are gone. **~370 LoC of bespoke SQL deleted;
  no behaviour lost.**

### 1.3 The relationship today

```
┌────────────────────────── eduport (the app) ──────────────────────────┐
│                                                                       │
│  frontend/  ──────►  eduport-tauri  ──────►  eduport-core             │
│   (Svelte 5)         (Tauri commands)        (domain layer)           │
│                                                   │                   │
│                                                   ▼                   │
│                                          ┌────────────────────┐       │
│                                          │  vaultdb-core      │       │
│                                          │  (library)         │       │
│                                          │  Vault, Query,     │       │
│                                          │  LinkGraph,        │       │
│                                          │  mutations         │       │
│                                          └────────────────────┘       │
│                                                                       │
└───────────────────────────────────────────────────────────────────────┘
```

eduport's `Cargo.toml` depends on vaultdb-core **as a path
dependency** today, and will switch to a crates.io dependency once
the `CARGO_REGISTRY_TOKEN` is configured at the GitHub-actions level.

**This is the right shape.** It looks like a real OS-stack diagram
because it is one. The library doesn't know what an "entity type"
is. The app doesn't know how to write atomic-rename mutations.

---

## 2. The big idea, audited

You said: *"I would like to have an easy to plug tool that turns
markdown into a database. People will own their data, just in
markdown. They will be able to query it, modify it etc. Think
SQLite but with markdowns. It should scale well, but does not need
to be something production scale, again, think sqlite."*

### 2.1 What "SQLite for Markdown" promises and what it doesn't

The SQLite analogy is **structurally apt** but **technically
ambitious.** Let me break it down honestly:

| SQLite property | vaultdb today | Gap |
|---|---|---|
| Single-file, no server | ✅ — files-on-disk, no daemon | — |
| Embeddable as a library | ✅ — `vaultdb-core` is a crate | — |
| Stable file format | ⚠️ — markdown + YAML are standard, but the *vault layout* (`.vaultdb/lock`, `.obsidian/` marker) is not formally specified | Write a SPEC.md |
| Decades of bug-fixing | ❌ — v1.0.0 published, months old | Time + adoption |
| Public benchmarks | ❌ — no published numbers | Add `cargo bench` and a SCALE.md |
| Multiple language bindings | ✅ — pyo3 + wasm shipped | More targets later |
| Crash-safe | ✅ — flock + journal + atomic rename | Already done |
| ACID transactions | ⚠️ — per-file atomicity only; no multi-file ACID across the vault | Possibly fine — but call it out |
| Public test suite | ✅ — 182 tests in CI | — |
| Acceptance as "the obvious choice" | ❌ — single known consumer | Need ≥1 more |

**The honest framing:** vaultdb is positioned correctly to *become*
SQLite-for-markdown. It is not there yet. The codebase is good
enough that getting there is a question of **time, adoption,
benchmarks, and one or two more consumers** — not a question of
ripping anything up.

### 2.2 Where the analogy gets hard

SQLite has one big thing vaultdb structurally cannot have: a single
binary format with a single canonical engine. Markdown vaults have
*multiple* legitimate engines (Obsidian, Logseq, Foam, Dendron,
Quartz). Your library's job is **interop**, not ownership. That's
a strength, but it changes the strategy:

- SQLite's competitive moat is "the format is the library."
  vaultdb's moat must be "the *queries* are the library." Other
  tools (Obsidian, Logseq) read/write the same files; what makes
  vaultdb worth depending on is the **query layer**, the
  **mutation safety**, and the **embeddability**.
- This means **the where-DSL is your product surface,** not the
  internal AST. People will write their queries in your DSL and
  expect it to keep working across versions. Treat the DSL as a
  formal artefact — version it, deprecate carefully, document its
  grammar.

---

## 3. Direction: where this can go

### 3.1 The "takeoff" path

If you want vaultdb to be **a thing people pick up**, the next 3–6
months matter more than the last 6 did. Concretely:

**A. Get a second consumer.** Anything — even a toy. Suggestions:

- A static-site generator (`vaultdb-ssg`) that turns a vault into
  HTML using `Vault::query()` for index pages
- A **CLI dashboard** (`vault-stats`) that renders vault metrics to
  the terminal
- A **VS Code extension** that uses `vaultdb-wasm` to evaluate
  where-clauses against the open workspace
- A **personal Zettelkasten viewer** (small, fast, opinionated)

The point isn't the product. The point is to expose API
ergonomic problems that eduport's needs hide. Until you've used
vaultdb-core in two contexts, you don't know what's actually
general.

**B. Publish benchmarks.** Three numbers matter:
- 1k records — query latency
- 10k records — query latency, link-graph build time
- 100k records — does it still finish in <1s for a simple filter?

Put these in `BENCHMARKS.md`. Update them on every release. SQLite
is trusted because they publish numbers; you should too.

**C. Add FTS to vaultdb itself (opt-in).** This is the big one.
Right now, every consumer that wants real search has to do what
eduport did — maintain a parallel SQLite database with FTS5. That's
a high adoption tax. Two ways to handle it:

- **Inside vaultdb:** `vaultdb-fts` crate with a `FtsIndex` that
  consumes vault events and exposes a `Vault::search_fts(query)`
  API. *Opt-in* — vaultdb-core stays pure.
- **In a separate crate:** publish `vaultdb-fts` so eduport (and
  others) can drop their bespoke index and use the shared one.

Either way, eduport's `crates/eduport-core/src/index/` becomes
~2,200 LoC of dead code that you can delete from eduport and
maintain once, in vaultdb.

**D. Write the SPEC.** A document that says: "A vault is a
directory containing markdown files with YAML frontmatter. The
following filenames and folders are reserved: `.vaultdb/`,
`.obsidian/`, `.eduport/`. Records are identified by …" Without
this, no one can write a competing implementation, and "your data,
your files" rings hollow.

**E. Stability commitments.** Add `VERSIONING.md` that says: "After
v1.0.0, breaking changes require a major-version bump. Deprecated
APIs survive at least one major version. The where-DSL grammar is
versioned independently and will not change in incompatible ways
within a major version." Read it from the README. Without this,
adopters can't budget for upgrades.

### 3.2 What can go wrong

Honest enumeration of the risks:

**R1 — Scope creep into eduport-shaped concerns.** This is the #1
killer of "library + first app" projects. Right now your discipline
is good (the `eduport-type/` tag convention lives in eduport, not
vaultdb), but pressure will mount. Examples of what NOT to absorb
into vaultdb-core:

- Typed entity systems (eduport's 8 types)
- Schema validation rules
- Email parsing
- Watcher events

If anyone proposes these, the answer is: "those are domain
concerns, they live in the consumer crate." `ARCHITECTURE.md`
already says this. Keep saying it.

**R2 — The performance cliff.** vaultdb's "every read traverses the
filesystem fresh" promise is beautiful at 100 records. At 10k it's
sub-second on SSD. At 100k it's seconds — possibly tens of seconds
for queries that need the link graph (because the graph is rebuilt
from scratch). If a consumer adopts vaultdb for a vault that grows,
they'll hit a wall.

Mitigation: an **opt-in cache** layer. Not in vaultdb-core (which
stays cache-free) — as a separate `vaultdb-cache` crate that
implements a smart invalidation strategy keyed on mtime. Consumers
opt in when they need it. eduport would adopt it.

**R3 — Single-consumer trap.** Already covered. The longer eduport
is the only consumer, the more its specific needs warp the API.

**R4 — Markdown's fuzziness.** vaultdb assumes YAML frontmatter
between `---` lines. Real-world markdown has variants (TOML
frontmatter in Hugo, JSON in some tools, no frontmatter at all in
plain notes). Decide explicitly: do you support these or not? If
not, say so loudly. If yes, that's real work.

**R5 — The Obsidian dependency.** `Vault::discover()` looks for
`.obsidian/` as the vault marker. This conflates "this directory
is a vault" with "Obsidian uses it." Two consumers using vaultdb
on the same machine will fight over what `.obsidian/` means. A
neutral marker (`.vaultdb/config.toml`?) would decouple this.

**R6 — Concurrency model mismatch.** vaultdb uses a process-wide
advisory `flock`. This is fine for desktop apps. It fails on web
servers (many threads, no inter-process awareness) and on iCloud /
Dropbox sync (flock isn't honoured). Consumers will assume "ACID"
and get surprised. Document this clearly *and* consider an in-process
RwLock layer for server use cases.

**R7 — The MCP detour.** `vaultdb-mcp` is a real, working MCP
server (782 LoC). But MCP is still a young protocol; if it
fragments or gets superseded, you've paid maintenance cost for a
binding nobody uses. Not catastrophic — but be honest that it's a
*bet*, not a core feature.

**R8 — Maintenance burden of bindings.** pyo3 and wasm bindings
require keeping multiple toolchains green. Today the bindings are
*read-only* (pyo3) or *parser-only* (wasm). Expanding them to full
mutation support multiplies the test matrix. Decide whether the
ROI justifies it before each expansion.

---

## 4. The cleanup plan

Both repos are in genuinely good shape — there's no rot to scrub
out. What follows is **proactive hygiene** that prevents rot from
appearing as the codebases grow. Treat it as one-week's worth of
work spread across two weeks of evenings, not a sprint.

### 4.1 vaultdb sweeps

| Sweep | What | Estimated effort |
|---|---|---|
| **V1. SPEC.md** | Write a formal vault-format specification: directory layout, reserved names, frontmatter rules, link syntax, error semantics. | 1 day |
| **V2. BENCHMARKS.md** | Add `cargo bench` with three workloads (1k / 10k / 100k records). Publish numbers in README. Run on CI on tagged releases. | 1–2 days |
| **V3. VERSIONING.md** | Write the semver + deprecation policy. Link from README. | 2 hours |
| **V4. Where-DSL grammar version** | Add a `WHERE_DSL_VERSION` constant. Document grammar in `docs/WHERE_DSL.md`. Add backwards-compat tests that pin a corpus of valid queries from v1.0. | 1 day |
| **V5. `.vaultdb/config.toml` marker** | Make vault discovery neutral. Keep `.obsidian/` as a fallback for compatibility. | Half a day |
| **V6. cargo-deny + cargo-audit in CI** | Add license + advisory gates. | 2 hours |
| **V7. Doc examples are run in CI** | `#[doc(test)]` everything. Catches doc rot. | Half a day |

### 4.2 eduport sweeps

| Sweep | What | Estimated effort |
|---|---|---|
| **E1. HANDOVER.md is now stale** | The "pending verification" section is now done (flat-root layout, vaultdb migration both shipped). Either rewrite as a living STATUS.md or delete entirely. | 1 hour |
| **E2. Grep for "sidecar" wording** | Frontend Status page (and possibly other UI strings) still uses "sidecar" — a term that means nothing post-Phase-10. Find-and-replace. | 30 minutes |
| **E3. Rust toolchain pin** | CI uses 1.95, dev uses 1.92 (per HANDOVER). Pin both via `rust-toolchain.toml`. | 30 minutes |
| **E4. Edition consistency** | core is on 2024, tauri on 2021. Align both — 2024 is fine. | 30 minutes |
| **E5. eduport-core publish** | Once vaultdb-core lands on crates.io with the right manifest, switch eduport-core's dep from path to version and re-enable the `cargo package` dry-run in CI. | 1 hour |
| **E6. Frontend test coverage** | Currently 6 test cases. The filter/keyboard/markdown utilities deserve more. Aim for ~30 cases covering the wire-format adapters, filter logic, and keyboard handlers. | 2 days |
| **E7. Tauri command integration tests** | Currently 0. Add a smoke-test harness that spins up `EduportState` against a tmpdir vault and round-trips ~10 commands. | 1 day |
| **E8. Settle the index module's future** | After V8 below, eduport's FTS5 index could be replaced by `vaultdb-fts`. Decide: keep it bespoke (and accept the duplication tax) or migrate when ready. Document the decision in eduport's README. | Decision, not work |

### 4.3 Cross-repo hygiene

| Sweep | What |
|---|---|
| **X1.** Pick a co-dev convention. Right now vaultdb and eduport sit in different parent directories. Either make them siblings (recommended) or document the actual layout in CLAUDE.md. |
| **X2.** Memory cleanup: the auto-memory `MEMORY.md` references phase numbers (1, 10, 11) from the rewrite plan; that vocabulary is now retired. Refresh. |
| **X3.** Decide where the FTS work happens (vaultdb-fts crate inside the vaultdb repo, or a separate repo). Recommendation: inside vaultdb, opt-in. |
| **X4.** Decide where the cache work happens (vaultdb-cache crate, opt-in). Recommendation: inside vaultdb. |
| **X5.** Make `cargo install vaultdb` produce a usable CLI on a fresh machine. Smoke-test on a Linux + macOS + Windows VM. |

---

## 5. Production-grade roadmap

Five dimensions matter. Today's score (subjective) and where to aim:

### 5.1 Stability — today 6/10
- ✅ v1.0.0 published, semver-ish
- ✅ Tests cover the public API
- ❌ No written deprecation policy
- ❌ No long-term-support commitment
- ❌ No documented "what breaks across major versions"

**To reach 9/10:** Ship `VERSIONING.md`, hold v1.x compatible for 12+ months, only break at v2.0 with a migration guide.

### 5.2 Performance — today 4/10
- ✅ Streaming queries (O(1) RAM in best case)
- ✅ Top-K optimization for sort+limit
- ❌ No published benchmarks
- ❌ No CI perf-regression tests
- ❌ Link graph rebuilt every call (no caching)
- ❌ No mtime-aware change detection

**To reach 8/10:** Publish `BENCHMARKS.md`. Add `cargo bench` to CI. Ship `vaultdb-cache` (opt-in). Add scale targets to docs (e.g. "comfortable up to 50k records on consumer SSDs").

### 5.3 Documentation — today 7/10
- ✅ ARCHITECTURE.md, README, RELEASE.md, SAFETY.md, CHANGELOG
- ✅ Module-level doc comments throughout
- ❌ No "Getting Started" tutorial
- ❌ No "Cookbook" with worked examples
- ❌ No "Migrating from grep + find" guide for new users
- ❌ Where-DSL grammar isn't documented separately

**To reach 9/10:** Tutorial. Cookbook. WHERE_DSL.md. Example projects. A docs site (mdbook is fine).

### 5.4 Ecosystem — today 5/10
- ✅ pyo3 + wasm bindings shipped
- ✅ MCP server shipped
- ✅ CLI shipped
- ❌ One known consumer (eduport)
- ❌ No "made with vaultdb" gallery
- ❌ No examples directory in the repo

**To reach 8/10:** Get a second consumer. Ship 3–5 small example projects (`examples/`). Publish a "Made with vaultdb" page on the README. Apply to Rust newsletters when v1.1 ships.

### 5.5 Releaseability — today 8/10
- ✅ Automated GitHub Actions publish to crates.io / PyPI / npm
- ✅ Per-crate publishing pipeline (RELEASE.md)
- ✅ CI gates (fmt + clippy + test + dry-run publish)
- ❌ No automated changelog generation
- ❌ No release notes template

**To reach 9/10:** `release-please` or similar. Release-note template in `.github/`.

---

## 6. Recommended sequence (next 6 weeks)

I'd order this way. Each step builds on the previous.

**Week 1 — Cleanup pass.**
- Do E1, E2, E3, E4, V3, V6, X2. None take more than an hour.
- These pay off forever and clear noise so the bigger moves are clean.

**Week 2 — Specs and policies.**
- V1 (SPEC.md) and V3 (VERSIONING.md) and V4 (where-DSL doc).
- These are the trust-building artefacts. Adopters read these.

**Week 3 — Benchmarks.**
- V2: write the bench harness, run it, publish numbers.
- This is the single biggest credibility multiplier. Without
  numbers, "scales well" is a claim. With numbers, it's a fact.

**Week 4 — Pick the FTS path.**
- Spike `vaultdb-fts` as an opt-in crate in the vaultdb repo.
- Migrate eduport's `crates/eduport-core/src/index/` to use it
  (or decide to keep eduport's bespoke version and document why).
- The deletion of ~2,200 LoC in eduport is the win signal.

**Week 5 — Second consumer.**
- Pick the smallest possible one (CLI dashboard? mdbook plugin?
  Zettelkasten viewer?). Build it in a weekend. Publish it.
- Notice every API friction point. File issues against vaultdb.
- Fix them in v1.1.

**Week 6 — Ship v1.1 with the lessons.**
- Release notes. Updated benchmarks. Migration guide if any
  breaking changes (avoid if possible — save those for v2).
- Submit to Rust newsletter, HackerNews "Show HN", Reddit r/rust.
- This is your launch.

---

## 7. What I think

You're not doing something wrong. You're doing something
**unfashionably right**: building a small, sharp library before
the big-bang app on top. That's the SQLite playbook. It's also
the boring playbook, which is why most projects skip it.

The specific call you made — "I don't want to do one thing in two
different places" — is the kind of architectural instinct that
separates projects that grow from projects that calcify. Today's
substrate migration was that instinct paying off: ~370 LoC of
duplicated SQL deleted, zero behaviour lost.

The biggest single risk is **staying solo for too long.** Right
now eduport's needs and vaultdb's API are co-evolved. That's
fine — it's how good libraries get bootstrapped. But within
3 months, vaultdb needs to be picked up by something that is *not*
eduport, even if that something is a 200-line example project.
Otherwise the API will silently warp toward eduport-shaped use
cases and the "general-purpose" framing will quietly stop being
true.

The second biggest risk is **the benchmarks gap.** "It scales
well" without numbers is a marketing claim. Adopters who are
evaluating libraries — *especially* the kind of careful adopters
you want, the ones who write production software — will not trust
that claim. Publish numbers. Even bad numbers ("comfortable at 10k,
slow at 100k, here's why and here's the workaround") are infinitely
better than no numbers.

Everything else — the cleanup, the polish, the bindings — is
maintenance. Important, but not strategic. Do it in the
background.

You have the right idea. You have a clean implementation. You
have one good consumer. You have two more years of execution
ahead of you. That's a healthy place to be on this kind of
project.

— Claude
