# Eduport — Design Spec

**Status:** draft for review
**Date:** 2026-05-06
**Author:** brainstormed with Claude

---

## 1. Overview

Eduport is a single-user desktop app for tracking university applications.
It manages a personal database of programs, universities, labs, people,
applications, documents, and free-form notes — all stored as Markdown files
with YAML frontmatter, in a sync-friendly folder layout.

The user is a single person applying to multiple programs across multiple
universities and countries. They use Obsidian for research notes and want
this app to live alongside that workflow without conflicting with it.

## 2. Goals and non-goals

### Goals

- **Track structured data** about programs, applications, and the people /
  labs / universities behind them.
- **Tag-driven filtering** as the primary discovery mechanism — the user has
  said this is the most important UX affordance.
- **Sync-safe** across two-to-three personal devices via folder sync (Dropbox
  / iCloud / Syncthing). No central server, no account.
- **Obsidian-friendly**: every record is a Markdown file with YAML frontmatter
  and Obsidian-style `[[wikilink]]` references. The user can edit any record
  directly in Obsidian; the app picks up changes via a file watcher.
- **Cross-platform desktop app** (macOS, Windows, Linux) with a real native
  window.

### Non-goals (v1)

- Multi-user, sharing, or collaboration.
- Cloud backend or accounts.
- Mobile app.
- **Email *sending* or *inbox parsing***. Eduport logs correspondence as
  Email entities (with `.eml` import); it does not connect to IMAP / SMTP /
  Gmail / Outlook. The user's existing email client remains in charge of
  actually sending and receiving.
- Calendar / OS-level reminders.
- Public publishing of any record.
- Auto-importing program data from external sources (university websites,
  Mastersportal, etc.).

## 3. Storage architecture

### 3.1 Folder layout

The user picks a single **data folder**. Inside it, the app expects three
top-level locations (all configurable in settings):

```
<data folder>/
├── *.md                    # all entity files, flat
├── attachments/            # binaries (PDF, docx, scans) — configurable
└── notes/                  # free-form .md, configurable
```

The data folder may live anywhere — inside an Obsidian vault, beside one,
or in an entirely separate folder synced via Dropbox / iCloud / Syncthing.

### 3.2 File naming

Every entity file is named:

```
<slug>-<id>.md
```

- `<slug>` is a kebab-case slug derived from the entity's name
  (`MSc CS (AI track)` → `msc-cs-ai-track`).
- `<id>` is a 4-character alphanumeric token, generated at creation time
  (`A4f2`, `K9p3`, …). The id is the stable handle; the slug is decorative
  and may change if the user renames.

**Slug generation rules** (deterministic, applied to the entity's name):

1. Unicode normalize to NFKD and strip combining marks (so `ETH Zürich` →
   `ETH Zurich`).
2. Apply a small transliteration table for European Latin characters that
   don't decompose under NFKD: `ø→o`, `Ø→O`, `æ→ae`, `Æ→AE`, `œ→oe`,
   `Œ→OE`, `ß→ss`, `þ→th`, `Þ→TH`, `ð→d`, `Ð→D`, `ł→l`, `Ł→L`. So
   `Søren Kierkegaard` → `Soren Kierkegaard`. Non-Latin scripts (Cyrillic,
   CJK, etc.) are not transliterated; they pass through to step 3 and end
   up as hyphens — acceptable v1 behaviour.
3. Lowercase.
4. Replace any run of non-alphanumeric characters with a single `-`.
5. Strip leading and trailing `-`.
6. Truncate to 60 characters at a word boundary.
7. If the result is empty (e.g., the name is purely emoji or non-Latin
   without enough ASCII letters), fall back to `untitled`.

The 4-char id uses the alphabet `[a-zA-Z0-9]`, giving 62⁴ ≈ 14.7M
combinations — collisions are vanishingly unlikely. On creation, the app
checks the data folder and retries the id if a collision occurs.

### 3.3 Type discrimination

Entity type is encoded as a hierarchical tag, not via folder placement:

| Tag | Entity |
|---|---|
| `eduport-type/university` | University |
| `eduport-type/lab` | Lab |
| `eduport-type/person` | Person |
| `eduport-type/program` | Program |
| `eduport-type/application` | Application |
| `eduport-type/document` | Document |
| `eduport-type/email` | Email |

Document subtype uses a parallel namespace:

| Tag | Document subtype |
|---|---|
| `eduport-doctype/cv` | CV |
| `eduport-doctype/sop` | Statement of purpose |
| `eduport-doctype/transcript` | Transcript |
| `eduport-doctype/recommendation` | Recommendation letter |
| `eduport-doctype/portfolio` | Portfolio |
| `eduport-doctype/other` | Other |

The notes folder contains free-form `.md` and is **not** subject to
`eduport-type/*` discipline. Files there are surfaced under "Notes" in the
UI but otherwise opaque to the app.

### 3.4 Cross-references

References between entities use Obsidian wikilinks:

```yaml
university: "[[eth-zurich-K9p3]]"
people:
  - "[[jane-doe-A4f2]]"
  - "[[bob-smith-G7h1]]"
```

This makes the Obsidian graph view work natively, gives backlinks for free,
and survives renaming inside Obsidian (which auto-rewrites references on
rename). The app parses `[[…]]` strings out of YAML when building its index.

**Resolution algorithm.** When the app encounters a wikilink `[[foo-K9p3]]`,
it resolves in this order:

1. Exact filename match: a file named `foo-K9p3.md` in the data folder.
2. Id-suffix fallback: if (1) misses, find any `*-K9p3.md` and treat that
   as the target. This catches the case where the user renamed the slug
   part outside Obsidian (so Obsidian didn't auto-rewrite references).
3. If neither matches, the link is rendered as broken in the UI and the
   index records it as such.

The 4-char id is therefore a stable handle, not just a collision-avoidance
suffix.

### 3.5 Tags

Tags are a **single global namespace**. The same tag (`ai`, `theory`,
`switzerland`) can appear on any entity type. The UI's tag picker offers
autocomplete from existing tags across the whole vault.

The `eduport-type/*`, `eduport-doctype/*` and any other reserved-prefix tags
are managed by the app. User-facing filter chips display short user tags
(`ai`, `scholarship`) and hide reserved-prefix ones unless explicitly
included.

### 3.6 Storage layer (SQLite cache)

Markdown files are the **source of truth**. SQLite is a derived per-machine
index. To prevent the index from being synced (sync services corrupt SQLite
on concurrent access), it lives **outside** the data folder, in the OS's
per-user cache directory:

| OS | Path |
|---|---|
| macOS | `~/Library/Caches/Eduport/<data-folder-hash>.sqlite` |
| Linux | `~/.cache/eduport/<data-folder-hash>.sqlite` |
| Windows | `%LOCALAPPDATA%\Eduport\Cache\<data-folder-hash>.sqlite` |

The `<data-folder-hash>` is a short hash of the configured data folder path,
so users with multiple data folders get separate caches. The Python sidecar
uses `platformdirs` to resolve the right base path per OS.

On launch:
1. The Python sidecar runs a quick mtime reconciliation against the SQLite
   index — any `.md` file newer than its index entry gets re-parsed.
2. A `watchdog` file watcher subscribes to the data folder, attachments
   folder, and notes folder.
3. UI queries hit SQLite for sub-millisecond filtering.

If the SQLite file is corrupted, missing, or out-of-date by an unreasonable
margin, the app rebuilds it from scratch. At expected scale (≤ 1000 records)
a full rebuild finishes in well under a second.

### 3.7 Sync conflicts

Because all entity data lives in plain text, sync conflicts are handled by
the sync service exactly as they would be for any text file: a "conflicted
copy" gets created in the same folder. The user resolves conflicts in their
text editor of choice (Obsidian, VS Code), and the app's file watcher picks
up the resolved file.

### 3.8 Parse-error handling

When the file watcher fires on a `.md` file the parser cannot understand
(invalid YAML, missing required fields, malformed wikilinks), the app does
**not** crash and does **not** silently drop the file from the index.
Instead:

1. The parser captures the error (file path, line if available, message).
2. The SQLite `parse_errors` table records `(path, error, occurred_at)`.
3. The previous successfully-parsed version of the entity (if any) stays
   in the index — the index reflects last-known-good state, not last-known-
   exists.
4. The UI shows a "Files with errors (N)" badge in the sidebar. Clicking
   opens a list with each broken file, the error, an "Open in Obsidian"
   button to fix it, and a "Re-parse now" button.
5. Once the file parses cleanly, its `parse_errors` row is removed and
   the index is updated.

This means a typo in YAML never makes data disappear — it just stops the
update flow until you fix it.

## 4. Data model

All examples below use YAML frontmatter; the body of each `.md` file is
free-form Markdown.

### 4.1 University

```yaml
---
tags:
  - eduport-type/university
  - switzerland
name: "ETH Zurich"
country: "Switzerland"
city: "Zurich"
website: "https://ethz.ch"
links:
  - label: "Admissions"
    url: "https://ethz.ch/students/en/studies/applications.html"
emails:
  - label: "General admissions"
    email: "studiensekretariat@ethz.ch"
---

Notes about ETH go here.
```

### 4.2 Lab

```yaml
---
tags:
  - eduport-type/lab
  - ai
  - theory
name: "Machine Learning Group"
focus: "Foundations of deep learning"
website: "https://ml.ethz.ch"
university: "[[eth-zurich-K9p3]]"
links:
  - label: "Publications"
    url: "https://ml.ethz.ch/publications.html"
emails:
  - label: "Lab admin"
    email: "ml-admin@inf.ethz.ch"
---

Lab description.
```

### 4.3 Person

```yaml
---
tags:
  - eduport-type/person
  - ai
name: "Jane Doe"
role: "Professor"
email: "jane.doe@inf.ethz.ch"
website: "https://janedoe.example"
university: "[[eth-zurich-K9p3]]"
labs:
  - "[[ml-group-B2n4]]"
links:
  - label: "Google Scholar"
    url: "https://scholar.google.com/..."
  - label: "Twitter"
    url: "https://twitter.com/..."
---

Met at NeurIPS 2025. Works on alignment.
```

### 4.4 Program

```yaml
---
tags:
  - eduport-type/program
  - ai
  - theory
name: "MSc Computer Science (AI track)"
level: "masters"               # undergrad | masters | phd
department: "D-INFK"
language: "English"
duration: "2 years"
deadline: "2026-12-15"
tuition: "CHF 1,460/yr"
website: "https://inf.ethz.ch/studies/master/master-cs.html"
university: "[[eth-zurich-K9p3]]"
people:
  - "[[jane-doe-A4f2]]"
  - "[[bob-smith-G7h1]]"
links:
  - label: "Program page"
    url: "https://inf.ethz.ch/studies/master/master-cs.html"
  - label: "Application portal"
    url: "https://www.ethz.ch/apply"
emails:
  - label: "AI track lead"
    email: "jane.doe@inf.ethz.ch"
    person: "[[jane-doe-A4f2]]"
---

Strong AI track with mandatory thesis.
```

### 4.5 Application

```yaml
---
tags:
  - eduport-type/application
  - 2026-cycle
program: "[[msc-cs-ai-track-Q7w8]]"
status: "drafting"             # planning | drafting | submitted | decision-pending | accepted | rejected | withdrawn
internal_deadline: "2026-12-01"
submitted_at: null
decision_at: null
documents:
  - "[[cv-2026-03-T8d2]]"
  - "[[sop-eth-msc-ai-X4k2]]"
---

Strong fit. Reach out to [[jane-doe-A4f2]] before submitting.

- [x] Request transcript by 2026-10-01
- [ ] Draft SOP by 2026-11-15
- [ ] Submit by 2026-12-15
```

The body's checkbox lines (`- [ ]` / `- [x]`) with optional date suffixes
are parsed by the app and surfaced in the global Deadlines view. Toggling a
checkbox in the app's detail panel writes back to the file.

### 4.6 Document

```yaml
---
tags:
  - eduport-type/document
  - eduport-doctype/cv
title: "CV (March 2026)"
date: "2026-03-15"
file: "attachments/cv-2026-03-T8d2.pdf"   # path relative to data folder
---

Latest CV — added research section.
```

For recommendation letters specifically, an extra `recommender` field links
to the Person:

```yaml
---
tags:
  - eduport-type/document
  - eduport-doctype/recommendation
title: "Rec letter from Jane Doe"
date: "2026-11-10"
file: "attachments/rec-jane-doe-2026-11-N3p7.pdf"
recommender: "[[jane-doe-A4f2]]"
---
```

**Document status (optional).** A Document can be created *before* the
binary exists — typical for tracking a recommendation letter you've
requested but not yet received. The optional `status` field captures this:

```yaml
---
tags:
  - eduport-type/document
  - eduport-doctype/recommendation
title: "Rec letter from Jane Doe (requested)"
status: "requested"           # requested | drafting | received
requested_at: "2026-10-01"
recommender: "[[jane-doe-A4f2]]"
# file: omitted until the letter is received
---

Asked by email on 2026-10-01. Soft deadline 2026-11-15.
```

When the letter arrives: add the `file:` field, change `status` to
`received` (or remove the field — `received` is the default when `file` is
present and `status` is unset). The Documents list view filters by status,
so "outstanding recommendations" is one click away.

**One-sided links.** A Document does not store an `applications:` list. The
Application's `documents:` field is the single source of truth for that
relationship. The Document's detail panel shows "Used in:" via the app's
backlinks index, derived from scanning all Applications. This avoids the
drift problem of keeping bidirectional fields in sync across edits.

File metadata (size, type, mtime) is computed on-demand from the binary on
disk, not stored in frontmatter.

### 4.7 Email

A logged record of an email sent or received. One file per logged email.
The body of the `.md` file holds the full email content (or a paste of the
relevant portion).

```yaml
---
tags:
  - eduport-type/email
  - 2026-cycle
direction: "outbound"             # outbound | inbound
date: "2026-09-20"
subject: "Question about MSc CS deadline extension"
from: "rusen@example.com"
to:
  - "admissions@inf.ethz.ch"
cc:
  - "jane.doe@inf.ethz.ch"
related_program: "[[msc-cs-ai-track-Q7w8]]"
related_application: "[[2026-eth-msc-ai-M5j6]]"
related_people:
  - "[[jane-doe-A4f2]]"
in_reply_to: "[[email-2026-09-18-eth-X8k4]]"   # optional, for threading
attachments:
  - "[[cv-2026-03-T8d2]]"        # links to Document entities
---

Hi,

I'm planning to apply to the MSc CS program for the 2026 cycle...
(full email body, copy-pasted)
```

Filename pattern: `email-<YYYY-MM-DD>-<short-subject-slug>-<id>.md`.

**Threading.** The `in_reply_to` field is a wikilink to a previous Email
entity. The detail panel shows the chain of replies (forward and backward).
v1 follows `in_reply_to` only one level in each direction; deep threading
is deferred.

**`.eml` import.** The user can drag a `.eml` file (exported from any email
client — Apple Mail, Outlook, Thunderbird, Gmail "Show original → save")
onto the app window. The Python sidecar parses it (`email.parser` from
stdlib) and opens a pre-filled form:

- `from`, `to`, `cc`, `subject`, `date` — populated from headers.
- `direction` — inferred from whether the user's address (configurable in
  settings) appears in `from` (outbound) or `to`/`cc` (inbound).
- Body — copied as plaintext / converted to markdown if the email is HTML.
- `related_*` and `attachments` — left for the user to fill in via wikilink
  pickers.

After save, the `.eml` itself is **not** retained — only the parsed Email
entity. (If the user wants the original, they can save it as an attachment
under the `attachments/` folder and link it via the `attachments:` field.)

**Privacy note.** Full email bodies live in the data folder, which is
synced. Sync providers can see this content. This is the same exposure as
the rest of the data folder; mention this clearly in onboarding.

**Cross-cutting integration.**

- Application detail panels show a "Communications" section listing all
  Emails with `related_application` pointing at the Application, in
  chronological order.
- Person detail panels show a "Recent emails" section.
- Program detail panels show a similar section.
- These sections are derived via backlinks; Emails own the link, the other
  entities don't store a reverse list.

### 4.8 Free-form notes

A free-form note is any `.md` in the `notes/` folder. The frontmatter is
optional. The app surfaces notes in a "Notes" sidebar entry and resolves
wikilinks both ways: a Program can wikilink to a note, and the note's
detail page shows backlinks.

## 5. Tech stack

| Layer | Choice | Rationale |
|---|---|---|
| Native shell | **Tauri** (Rust) | Cross-platform, ~5MB binaries, strong packaging story. |
| Frontend | **Svelte** + **SvelteKit** | Lowest boilerplate, Tauri's first-class pairing. |
| IPC | **HTTP loopback** (127.0.0.1, random port) | Backend can run independently during dev; easy to debug. |
| Sidecar | **FastAPI** + **uvicorn** | Async, Pydantic-native, familiar. |
| Models | **Pydantic v2** | Schema, validation, serialization in one. |
| File watcher | **watchdog** | Cross-platform abstraction over inotify / FSEvents / ReadDirectoryChangesW. |
| Markdown (parse) | **markdown-it-py** | CommonMark + GFM, matches Obsidian's parser. Used by the sidecar for indexing and rendering. |
| Markdown (edit) | **CodeMirror 6** | In-app body editor. Markdown mode + small wikilink-autocomplete extension. |
| Storage | **SQLite via stdlib** + **FTS5** | No ORM at v1; schema is small enough for direct SQL. FTS5 (built into stdlib's `sqlite3`) provides full-text body search. |
| Tooling | **uv** | User's standard. |

### Packaging (deferred)

The Python sidecar will be bundled into the Tauri binary using **PyInstaller**
(or PyOxidizer if it produces smaller binaries). Tauri's
`tauri.bundle.externalBin` embeds the resulting executable. Per-OS
installers (`.dmg`, `.msi`, `AppImage`) come from `tauri build`. This is
real work but deferrable until v1 is functional locally.

## 6. App architecture

```
┌─────────────────────────────────────────┐
│ Tauri shell (Rust)                      │
│  • spawns + supervises Python sidecar    │
│  • hosts WebView pointing at sidecar URL │
│  • bridges native dialogs (open / save)  │
│  • bridges "reveal in file manager"      │
└─────────────┬───────────────────────────┘
              │ HTTP loopback
              ▼
┌─────────────────────────────────────────┐
│ Python sidecar (FastAPI + uvicorn)      │
│  • REST API for entities + queries       │
│  • file watcher (watchdog)               │
│  • parser (YAML + markdown-it-py)        │
│  • SQLite indexer                        │
│  • write-through saves to .md files      │
└─────────────┬───────────────────────────┘
              │
   ┌──────────┴──────────┐
   ▼                     ▼
.md files             SQLite cache
(canonical)           (.eduport-index.sqlite)
```

The Tauri shell handles three things the WebView can't:
1. Native open/save dialogs (`tauri-plugin-dialog`).
2. Reveal-in-file-manager (small per-OS Rust commands).
3. Lifecycle of the Python sidecar (start, kill, restart on settings change).

Everything else — list views, filters, forms, markdown rendering — happens
in the WebView, calling into the Python REST API.

### 6.1 Bootstrap sequence

On launch:

1. Tauri shell selects a free local port and spawns the Python sidecar
   (bundled binary in production, or `uvicorn` directly in dev) bound to
   `127.0.0.1:<port>`.
2. Tauri polls `GET /health` on the sidecar's port every 100ms.
3. When `/health` returns 200, Tauri loads the WebView and points it at
   `http://127.0.0.1:<port>/`.
4. **Timeout: 5 seconds.** If `/health` never returns, Tauri retries the
   spawn once (different port), then surfaces a fatal error UI: "Eduport
   could not start its backend." with a link to view the sidecar log file.
5. On Tauri shell exit, the sidecar process is killed (Tauri owns its
   lifecycle).

The sidecar writes its log to the OS log directory (resolved via
`platformdirs.user_log_dir`), with rolling files (10 MB × 3). The error UI
in step 4 deep-links to this file.

## 7. UI design

### 7.1 Three-pane layout

```
┌────────┬─────────────────────────┬──────────────┐
│ Sidebar│  Top bar (search, +new) │              │
│        ├─────────────────────────┤   Detail     │
│ - Dash │  Filter chips           │   panel      │
│ - Dead │  ┌───────────────────┐  │              │
│ - Prog │  │  List / Kanban    │  │  - fields    │
│ - App  │  │                   │  │  - actions   │
│ - …    │  │                   │  │  - body      │
│        │  │                   │  │  - checkboxes│
│ Tags   │  └───────────────────┘  │              │
└────────┴─────────────────────────┴──────────────┘
```

- **Sidebar** (left, ~220px): nav items grouped into Workspace (Dashboard,
  Deadlines), Database (Programs, Applications, People, Universities, Labs,
  Documents, Notes), and Tags. Each entity nav item shows a count.
- **Main area** (center): top bar with global search (⌘K) and a contextual
  "+ New …" button; filter row showing active tag/level/country chips;
  the list (or kanban for Applications). Click a row → loads the detail panel.
- **Detail panel** (right, ~320px, collapsible): structured fields, action
  buttons, rendered markdown body with parsed checkboxes.

### 7.2 Editing model — hybrid

Three editing affordances per entity:

| Action | Used for | UX |
|---|---|---|
| **Edit form…** | Structured fields (frontmatter): tags, dates, references, links, emails | In-app form modal |
| **Edit body…** | Markdown body of the file | In-app **CodeMirror 6** editor with markdown mode, autocomplete for `[[wikilinks]]`, and a side-by-side preview toggle |
| **Open in Obsidian** | Heavy editing, when you want Obsidian's full plugin ecosystem | Launches Obsidian via URI scheme `obsidian://open?path=…` |

The in-app body editor is intentionally lightweight — it covers daily
edits ("update a note, fix a typo, add a checkbox") without demanding a
context switch to Obsidian. The Obsidian escape hatch stays for serious
prose work or for using vault-wide Obsidian features.

While editing in-app, the file watcher temporarily ignores writes from
this process to avoid feedback loops. Save commits to disk immediately
(no manual save button — autosave on blur or after 1s of inactivity).

Inline checkboxes in the body are clickable in the detail panel without
opening any editor — the app rewrites the `- [ ]` ↔ `- [x]` line in the
file directly. Status changes for Applications happen in-app (dropdown or
kanban drag).

### 7.3 Cross-navigation

Every wikilink rendered in any detail panel is clickable and navigates to
that entity's detail page. The detail page of any entity also shows a
"Linked from" backlinks list (e.g., a Person's page shows the Programs and
Labs that reference them).

### 7.4 Document actions

The Document detail panel includes three native-shell actions:

| Action | Behavior |
|---|---|
| **Open** | Launch the binary with the OS default app. |
| **Reveal in file manager** | Open Finder/Explorer/Files with the binary selected. |
| **Save copy as…** | Native save dialog; copy binary to chosen path. |

File metadata (filename, size, type, modified date) is shown above these
actions and computed live from disk.

### 7.5 Key views

- **Dashboard** (default home): upcoming deadlines (next 30 days), application
  pipeline by status, recent activity, outstanding recommendations
  (Documents with `status: requested`), recent emails.
- **Programs list**: filterable by tag, level, country, university.
- **Applications list / kanban**: list view + kanban toggle. Kanban columns
  are application statuses; cards drag between columns. Each card shows a
  recent-emails count badge.
- **People / Universities / Labs / Documents lists**: filterable by tag, plus
  type-specific filters (e.g., role for People, doctype/status for Documents).
- **Emails inbox**: chronological list of all logged Emails. Filters: direction
  (sent/received), date range, related Application, related Person. Group-by
  toggle: "By thread" follows `in_reply_to` chains, "By application" groups
  emails under their related Application.
- **Notes**: flat list of free-form notes with backlinks.
- **Deadlines**: chronological view of all parsed `- [ ]` items across the
  vault, plus all `deadline` fields on Programs / Applications, plus
  outstanding recommendation requests (`requested_at` minus a soft horizon).

### 7.6 Search

Global ⌘K opens a command palette. Two modes:

- **Quick lookup** (default): matches against entity names, tags, and free-form
  note titles. Sub-millisecond. Always shown first in results.
- **Full-text** (toggleable, or auto-engaged when the query is 3+ words): hits
  the SQLite FTS5 virtual table built from every entity's body content
  (markdown stripped of frontmatter). Returns matches with snippet highlights
  showing where the query matched.

The FTS5 index is maintained alongside the main SQLite index:

- On parse, the entity's body is inserted/updated in `entities_fts(rowid,
  body, name, tags)`.
- On entity delete, the corresponding FTS row is removed.
- Tokenizer: `unicode61` (default) with `remove_diacritics=2`, so searching
  "Zurich" matches "Zürich".

Searches are scoped by the active filter chips (tag, level, etc.) — if `ai`
is active, search results are limited to entities tagged `ai`.

### 7.7 First-run / empty state

On the very first launch (no `settings.toml` exists):

1. A welcome modal asks the user to pick a data folder via a native folder-
   picker dialog. Default suggested path: `~/Documents/Eduport`.
2. If the chosen folder is empty, the app creates `attachments/` and
   `notes/` subfolders.
3. The app offers to create a small set of sample entities ("Sample
   University", "Sample Program", "Sample Application") so the user can see
   the UI populated. The user can decline and start with a blank slate.
4. The app then opens to the Dashboard.

When viewing a list view that's empty (no programs, no applications, etc.),
the list shows a friendly empty state with a one-line description of the
entity type and a primary "+ New …" call-to-action.

### 7.8 Soft-delete

Deletion in the app is a two-step interaction:

1. Click "Delete" in the detail panel → confirm modal ("Move to trash?").
2. The file is moved to `<data folder>/.eduport-trash/<original-name>.md`
   via the `send2trash` Python library when possible (which uses the OS's
   own Trash where available), falling back to the in-folder `.eduport-trash/`
   directory.

Trashed files are excluded from the index and from all UI views.
A "Trash" entry in the sidebar (collapsible, hidden by default) lets the
user restore an item with one click.

The app does **not** automatically empty the trash. Manual "Empty trash"
button in the trash view, with a confirmation. Removing files outside the
app (e.g., directly in the file manager) is not undoable — the OS Trash is
the user's last line of defense in that case.

### 7.9 Logging and error visibility

Two layers:

- **Sidecar log file** (per-OS log dir, see §6.1) — rolling 10 MB × 3,
  contains every parse, watcher event, save, and error. Written by Python
  via `logging` stdlib. Useful for diagnosing problems after the fact.
- **In-app status panel** (sidebar entry, hidden by default, badge appears
  when there's something to show): shows recent errors, parse-error
  files (linked from §3.8), watcher reconnection events, and the path to
  the log file. Click an error → "Open in Obsidian" / "Re-parse" action.

When an unrecoverable error happens (sidecar dies, FTS index corrupted),
the UI surfaces a banner: *"Something went wrong. Click for details."*
Details include the exact error message and a "Copy log path" button.

## 8. Workflows

### 8.1 Add a new program

1. User clicks "+ New Program" in top bar.
2. Form modal opens with fields: name (required), level, university (wikilink
   picker autocompleting from existing Universities), department, language,
   deadline, website, tuition, links, emails, tags.
3. On save:
   - App generates `<slug>-<4chars>.md` from the name.
   - Writes YAML frontmatter + an empty body.
   - File watcher detects the file; SQLite index updates.
   - UI navigates to the new program's detail page.

### 8.2 Track an application

1. From a Program's detail page, click "+ New Application".
2. Form opens pre-filled with the program reference. User sets internal
   deadline, status (defaults to `planning`), and any starter tags.
3. After save, user adds checkboxes via the in-app body editor (§7.2) or
   in Obsidian.
4. Status changes via a dropdown in the detail panel or kanban drag.
5. As deadlines approach, they appear in the Dashboard and Deadlines view.

### 8.3 Filter by tag

1. Click any tag chip anywhere in the UI (sidebar, detail panel, list row).
2. The chip is added to the top-bar filter row.
3. The current list view is filtered. Filters persist as the user navigates
   between entity types — switching from Programs to People with `ai` active
   shows people tagged `ai`.
4. Click the X on a chip to remove it.

### 8.4 Find a document

1. Click "Documents" in sidebar.
2. Filter by `eduport-doctype/sop` to see all SOPs. (Filter combination is
   AND by default — see §12 open questions for the OR-toggle deferral.)
3. Click a row → detail panel shows file metadata and Open / Reveal / Save
   buttons.

### 8.5 Log an email (drag-and-drop or manual)

**Drag-and-drop path:**
1. User drags a `.eml` file from their email client onto the Eduport window.
2. Tauri's drop handler receives it; the sidecar parses headers and body.
3. A pre-filled "Log Email" form opens. User adds related Application,
   Program, People wikilinks via autocomplete pickers.
4. On save: a new Email entity is written; the original `.eml` is not
   retained.

**Manual path:**
1. From an Application or Person detail page, click "+ Log Email".
2. Empty form opens with `related_application` / `related_people`
   pre-filled based on context.
3. User pastes subject, body, and headers.
4. On save: same as above.

### 8.6 Fix a parse error

1. The sidebar shows "Files with errors (N)" badge.
2. Click → list of broken files with their error messages.
3. Click "Open in Obsidian" → user fixes the YAML.
4. File watcher detects the save → parser retries → if successful, the file
   leaves the error list and reappears in the index.

## 9. Settings

User-configurable, stored in the OS's per-user config directory as
`settings.toml` (resolved via `platformdirs`):

| OS | Path |
|---|---|
| macOS | `~/Library/Application Support/Eduport/settings.toml` |
| Linux | `~/.config/eduport/settings.toml` |
| Windows | `%APPDATA%\Eduport\settings.toml` |

```toml
data_folder = "/home/user/Documents/eduport-data"
attachments_folder = "./attachments"   # relative to data_folder
notes_folder = "./notes"               # relative to data_folder
theme = "system"                       # system | light | dark
user_email = "rusen@example.com"       # used for inbound/outbound inference on .eml import
```

The first run prompts for a data folder and the user's email address;
the others default as shown.

## 10. Test strategy

The implementation plan will detail what tests get written; this section
fixes the *shape* of testing so the plan doesn't have to invent it.

- **Sidecar (Python)** — `pytest`. Coverage targets:
  - Parser unit tests (YAML extraction, wikilink resolution, slug rules,
    parse-error capture, checkbox parsing).
  - Indexer integration tests (create / update / delete an entity file →
    SQLite reflects it).
  - File watcher tests using `tmp_path` and synthetic file events.
  - REST API contract tests with FastAPI's `TestClient`.
  - Pydantic model round-trip tests (YAML → model → YAML).
- **Frontend (Svelte)** — `Vitest` for component logic and store behavior.
  Keep coverage focused on data transformation and filter logic; visual
  regression is not in scope for v1.
- **End-to-end** — deferred to post-v1. The Tauri app is testable via
  WebDriver but the setup cost is significant; manual smoke testing of
  the bootstrap + key flows is enough at v1.

CI runs sidecar and frontend test suites on every commit. Pre-commit
hooks run `ruff` + `mypy` for Python and `eslint` + `svelte-check` for
the frontend.

## 11. Out of scope for v1

- **Conflict resolution UI** — sync-service handling is enough at single-user
  scale.
- **Automatic tag normalization** — case-folding, typo detection. The user
  should see typos quickly via the tag autocomplete.
- **Automatic data import** from external program databases.
- **Templates** — pre-filled forms for common program shapes. Add later if
  the user repeatedly fills the same fields the same way.
- **Plugin / extension API.**
- **Mobile or web access.**
- **Email *sending* / inbox parsing** (see §2). Eduport logs emails; it
  doesn't replace your email client.
- **End-to-end test automation** — manual smoke tests at v1.
- **Application "cycle" entity** — using a tag like `2026-cycle` works
  fine. Promote to entity if and only if the tag-based approach feels
  thin.

## 12. Open questions

These are minor. None block v1.

1. **Filter combination logic.** AND across all chips for v1. OR toggle if
   needed.
2. **Tag deletion.** What happens when a tag stops being used on any entity?
   Auto-disappears from sidebar (it's derived). What about a tag that's
   important but currently unused? Out of scope for v1 — can be addressed
   with a "pinned tags" config field if it becomes a problem.
3. **HTML-email body conversion.** `.eml` import for HTML emails: convert
   to markdown via `markdownify`, or store HTML in a fenced block? Default
   to `markdownify` with the fenced-block option as fallback when conversion
   fails.

## 13. v1 milestone

A working v1 that ships:

- Tauri shell + Python sidecar wired up via HTTP loopback (with the §6.1
  bootstrap sequence).
- All 8 entity types readable / writable: University, Lab, Person, Program,
  Application, Document, Email, Notes.
- 3-pane layout with sidebar nav.
- List views for each entity type, with tag and type-specific filters.
- Detail panel with structured-field display + Edit form modal +
  in-app body editor (CodeMirror) + Open in Obsidian.
- Cross-navigation via clickable wikilinks; backlinks panels.
- Document actions (open / reveal / save copy / status field for pending).
- Inline-checkbox toggling.
- Application kanban view.
- Email inbox view + `.eml` drag-and-drop import.
- Dashboard with upcoming deadlines + application pipeline + outstanding
  recommendations.
- Global ⌘K search with quick-lookup and full-text (FTS5) modes.
- First-run onboarding (§7.7) and empty states.
- Soft-delete with `.eduport-trash/` and OS-trash integration (§7.8).
- Parse-error sidebar (§3.8) and logs/status panel (§7.9).
- Settings for data folder paths and user email.
- Test suites: pytest (sidecar) + Vitest (frontend).
- Single-machine usage (packaging + multi-machine sync verification deferred).

Anything beyond this list — packaging installers, theme polish,
end-to-end test automation, etc. — is post-v1.
