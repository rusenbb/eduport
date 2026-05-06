# Eduport Frontend — Implementation Plan (Plan 2)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Svelte frontend that consumes the sidecar's REST API. End-state: a runnable Vite dev server (`npm run dev` inside `frontend/`) that, when pointed at a running `eduport-sidecar`, lets you list/filter/create/edit/delete entities, search, toggle checkboxes, view backlinks, manage tags, and use the kanban view for Applications. No Tauri shell yet — that's Plan 3.

**Architecture:** SvelteKit in SPA mode (`adapter-static`) with TypeScript. Single source of truth for sidecar URL via Vite env (`VITE_SIDECAR_URL`, default `http://127.0.0.1:8765`). Stores hold cached lists + active filters; routes are file-based; CodeMirror 6 powers the body editor.

**Tech Stack:** SvelteKit, TypeScript, Vite, Tailwind CSS, CodeMirror 6 (`@codemirror/lang-markdown`, `@codemirror/view`, `@codemirror/state`), `@uiw/codemirror-theme-...` (any dark/light theme), Vitest, eslint, prettier, svelte-check.

---

## File Structure

```
frontend/
├── package.json
├── svelte.config.js                # adapter-static
├── vite.config.ts
├── tsconfig.json
├── tailwind.config.cjs
├── postcss.config.cjs
├── src/
│   ├── app.html
│   ├── app.css                     # Tailwind + small global styles
│   ├── lib/
│   │   ├── api/
│   │   │   ├── client.ts           # typed fetch wrapper, env-driven base URL
│   │   │   ├── entities.ts         # list/get/create/update/delete
│   │   │   ├── search.ts
│   │   │   ├── checkbox.ts
│   │   │   ├── eml.ts
│   │   │   └── settings.ts
│   │   ├── types.ts                # mirrors sidecar Pydantic shapes (subset we use)
│   │   ├── stores/
│   │   │   ├── filters.ts          # active tag/level filters (persisted in URL)
│   │   │   ├── settings.ts         # cached settings, fetched once
│   │   │   └── status.ts           # parse-error count, watcher health for badge
│   │   ├── components/
│   │   │   ├── Sidebar.svelte
│   │   │   ├── TopBar.svelte
│   │   │   ├── FilterChips.svelte
│   │   │   ├── EntityList.svelte           # generic list, generates rows by entity type
│   │   │   ├── EntityRow.svelte            # one row, type-specific compact display
│   │   │   ├── DetailPanel.svelte
│   │   │   ├── DetailField.svelte          # one structured-field row
│   │   │   ├── BodyView.svelte             # rendered markdown + clickable wikilinks + checkbox toggle
│   │   │   ├── BodyEditor.svelte           # CodeMirror 6 wrapper
│   │   │   ├── EntityForm.svelte           # generic create/edit form, dispatched per entity type
│   │   │   ├── WikilinkPicker.svelte       # autocomplete dropdown for [[…]] field values
│   │   │   ├── TagPicker.svelte            # multi-tag input with autocomplete
│   │   │   ├── KanbanBoard.svelte          # for Applications
│   │   │   ├── KanbanCard.svelte
│   │   │   ├── EmptyState.svelte
│   │   │   ├── CommandPalette.svelte       # ⌘K
│   │   │   └── ErrorBanner.svelte          # parse errors, watcher disconnects
│   │   └── markdown.ts             # render markdown via marked.js with wikilink hooks
│   └── routes/
│       ├── +layout.svelte          # 3-pane shell
│       ├── +layout.ts              # initial settings + status fetch
│       ├── +page.svelte            # dashboard
│       ├── deadlines/+page.svelte
│       ├── [type]/+page.svelte     # generic list view per entity type
│       ├── [type]/[fileId]/+page.svelte    # detail
│       ├── notes/+page.svelte      # free-form notes
│       └── trash/+page.svelte
└── tests/
    ├── unit/
    │   ├── client.test.ts
    │   ├── filters.test.ts
    │   └── markdown.test.ts
    └── setup.ts
```

**Boundaries.** `lib/api/` is the only place `fetch` is called. `lib/stores/` are reactive caches. `lib/components/` are dumb-ish UI; they receive data via props or read from stores. `routes/` glue stores to layouts.

---

## Task 0: Scaffold SvelteKit project

**Files:** `frontend/*` (created by `npm create svelte@latest`).

- [ ] Create `frontend/` and run `npm create svelte@latest .` (TypeScript, ESLint, Prettier, Vitest, Playwright skipped). Pick "Skeleton project".
- [ ] Add deps: `npm install -D tailwindcss postcss autoprefixer @sveltejs/adapter-static && npm install marked codemirror @codemirror/lang-markdown @codemirror/view @codemirror/state @codemirror/theme-one-dark`
- [ ] Init Tailwind: `npx tailwindcss init -p`. Add `content: ['./src/**/*.{html,js,svelte,ts}']`. Import `@tailwind` directives in `src/app.css`. Import `app.css` from `+layout.svelte`.
- [ ] Switch to `adapter-static`: edit `svelte.config.js` → `import adapter from '@sveltejs/adapter-static'`, `kit.adapter = adapter({ fallback: 'index.html' })`. Add `prerender = false; ssr = false;` in `+layout.ts`.
- [ ] Smoke: `npm run dev` boots, `http://localhost:5173` renders the skeleton page.
- [ ] Commit: `chore(frontend): scaffold sveltekit + tailwind + codemirror deps`.

---

## Task 1: API client foundation

**Files:** `src/lib/api/client.ts`, `src/lib/types.ts`, `tests/unit/client.test.ts`.

- [ ] `client.ts`: export `apiFetch(path, init)` that prepends `VITE_SIDECAR_URL` (default `http://127.0.0.1:8765`), throws `ApiError` on non-2xx with the parsed JSON `detail` if present.
- [ ] `types.ts`: TypeScript types mirroring sidecar shapes — `EntityType`, `EntityListItem`, `EntityDetail`, `Application`, `Document`, `WikiLink`, etc. Keep narrow — only fields the UI uses.
- [ ] Unit test (vitest): mock fetch, verify `apiFetch` prepends base URL, parses JSON, throws `ApiError` on 4xx with `detail` message.
- [ ] Commit: `feat(frontend): typed fetch client with ApiError`.

---

## Task 2: Entity API module

**Files:** `src/lib/api/entities.ts`.

- [ ] Export `listEntities(type, tags?)`, `getEntity(type, fileId)`, `createEntity(type, frontmatter, body)`, `updateEntity(type, fileId, frontmatter, body)`, `deleteEntity(type, fileId)`.
- [ ] Each function uses `apiFetch` and returns the typed shape from `types.ts`.
- [ ] No tests — these are thin wrappers; behaviour is exercised by integration in Task 14+.
- [ ] Commit: `feat(frontend): entity CRUD API client`.

---

## Task 3: Search, checkbox, eml, settings API modules

**Files:** `src/lib/api/{search,checkbox,eml,settings}.ts`.

- [ ] `search.ts`: `search(q, limit?)` → list of `{file_id, type, name, snippet}`.
- [ ] `checkbox.ts`: `toggleCheckbox(fileId, line, checked)` → `{ok: true}`.
- [ ] `eml.ts`: `parseEml(file: File)` → multipart POST → returns `ParsedEml`.
- [ ] `settings.ts`: `getSettings()`, `putSettings(settings)`.
- [ ] Commit: `feat(frontend): search/checkbox/eml/settings API clients`.

---

## Task 4: Stores — filters + settings + status

**Files:** `src/lib/stores/{filters,settings,status}.ts`, `tests/unit/filters.test.ts`.

- [ ] `filters.ts`: a writable `{tags: string[], level?: string, country?: string}`. Helpers `addTag(t)`, `removeTag(t)`, `clear()`. Hook to URL search params via `goto` so filters survive reload.
- [ ] `settings.ts`: writable cache, populated once on layout load via `getSettings()`.
- [ ] `status.ts`: writable `{parseErrors: number, sidecarUp: boolean}`, polled every 10s via a tiny timer.
- [ ] Vitest for `filters.ts`: add/remove/clear behaviour, dedup logic.
- [ ] Commit: `feat(frontend): filters/settings/status stores`.

---

## Task 5: Markdown rendering with wikilink hooks

**Files:** `src/lib/markdown.ts`, `tests/unit/markdown.test.ts`.

- [ ] Export `renderMarkdown(body, onWikilinkClick)` → `{html, hooks}`. Use `marked` for parsing; post-process the output to replace `[[target]]` strings with `<a class="wikilink" data-target="...">target</a>`.
- [ ] Also extract a list of `Checkbox` lines (line number + checked state + text) — same shape as the sidecar's parser, for the toggle action.
- [ ] Unit test: round-trip a sample body containing a wikilink and a checkbox; assert HTML output and parsed checkboxes.
- [ ] Commit: `feat(frontend): markdown renderer with wikilink + checkbox hooks`.

---

## Task 6: Layout shell

**Files:** `src/routes/+layout.svelte`, `src/routes/+layout.ts`, `src/lib/components/{Sidebar,TopBar,FilterChips}.svelte`.

- [ ] `+layout.ts`: load → fetch settings, prime status, return `{settings, types}`.
- [ ] `+layout.svelte`: Tailwind grid `grid-cols-[220px_1fr_320px]` (the right pane is empty by default; routes that have a detail panel inject into a slot).
- [ ] `Sidebar`: hard-coded sections — Workspace (Dashboard, Deadlines), Database (one nav item per entity type with count fetched via `listEntities` lazily), Tags (top N), Settings link, Trash.
- [ ] `TopBar`: search input (opens command palette), contextual `+ New …` button (the route page provides `newAction` via context API), theme toggle stub.
- [ ] `FilterChips`: subscribes to `filters` store, renders chips with X button.
- [ ] Smoke: dev server shows three columns, sidebar nav links resolve.
- [ ] Commit: `feat(frontend): three-pane layout shell with sidebar + top bar`.

---

## Task 7: Generic list view at `[type]/+page.svelte`

**Files:** `src/routes/[type]/+page.svelte`, `src/lib/components/{EntityList,EntityRow,EmptyState}.svelte`.

- [ ] Route loads `listEntities(type, $filters.tags)`. Subscribe to `filters` store, refetch on change.
- [ ] `EntityList`: takes `items`, `type`, renders rows using `EntityRow` and headers.
- [ ] `EntityRow`: displays type-aware compact row — for Programs: name + level badge + university + deadline; for Applications: program name + status pill + internal_deadline; for People: name + role + university.
- [ ] Click a row → navigate to `[type]/[fileId]`. Selected row keeps a highlight.
- [ ] `EmptyState`: friendly message + "+ New …" CTA.
- [ ] Commit: `feat(frontend): generic list view with type-aware rows`.

---

## Task 8: Detail page at `[type]/[fileId]/+page.svelte`

**Files:** `src/routes/[type]/[fileId]/+page.svelte`, `src/lib/components/{DetailPanel,DetailField,BodyView}.svelte`.

- [ ] Route loads `getEntity(type, fileId)` (returns `entity`, `body`, `backlinks`).
- [ ] `DetailPanel`: header (name, breadcrumb), action buttons row (Edit form, Edit body, Open in Obsidian — Obsidian deferred to Plan 3, render as disabled or stub for now), structured fields, rendered body, backlinks list.
- [ ] `DetailField`: renders one frontmatter field. Wikilinks are clickable. Lists become chip rows. URLs become anchor tags. `links` and `emails` resource fields get their own compact list rendering with copy-to-clipboard buttons.
- [ ] `BodyView`: uses `markdown.ts`. Wikilink clicks navigate. Checkbox clicks call `toggleCheckbox` and optimistically flip locally.
- [ ] Backlinks block at the bottom of the panel: "Linked from N entities" with chip list grouped by `field`.
- [ ] Commit: `feat(frontend): entity detail page with body, fields, backlinks, checkbox toggle`.

---

## Task 9: Entity form (create + edit)

**Files:** `src/lib/components/{EntityForm,WikilinkPicker,TagPicker}.svelte`.

- [ ] `EntityForm`: dispatches per-entity sub-form by `type`. Each sub-form has its own field set matching the Pydantic model.
- [ ] `WikilinkPicker`: input with debounced autocomplete that hits `listEntities(targetType)` and shows matching names. Selecting an item stores the wikilink as `[[fileId]]`.
- [ ] `TagPicker`: input with autocomplete from a "all tags" list (fetched via `listEntities` aggregation — for now, query without filters and union the tags client-side; v2 can add a `/tags` endpoint).
- [ ] On submit: `createEntity` (when no file_id prop) or `updateEntity` (with file_id prop). On success, navigate to detail page.
- [ ] Modal opens via context API from `+ New …` button or detail panel's Edit-form button.
- [ ] Commit: `feat(frontend): entity form with wikilink + tag pickers`.

---

## Task 10: Body editor (CodeMirror)

**Files:** `src/lib/components/BodyEditor.svelte`.

- [ ] Mount CodeMirror 6 with `markdown()` extension and one-dark theme. Two-way bind a `value` prop. Autosave on blur or 1s of idle.
- [ ] Add a tiny wikilink-autocomplete extension: when user types `[[`, show a dropdown using the same source as `WikilinkPicker`.
- [ ] Edit button in `DetailPanel` toggles `BodyView` ↔ `BodyEditor`. On save, `updateEntity` is called with the same frontmatter and the new body.
- [ ] Commit: `feat(frontend): in-app body editor with wikilink autocomplete`.

---

## Task 11: Tag filter integration

**Files:** edit `EntityRow` and `DetailField` to make tags clickable; edit `Sidebar` to render top tags.

- [ ] Clicking a tag chip anywhere adds it to `filters` store (via `addTag(t)`).
- [ ] Sidebar shows the top 8 tags by count (computed across all entities — fetch `listEntities` per type once at layout load is too expensive; do it lazily on tag panel hover, or accept slight staleness).
- [ ] Clicking a sidebar tag also adds to filters.
- [ ] Commit: `feat(frontend): tag chips clickable across the UI`.

---

## Task 12: Command palette (⌘K) with quick lookup + FTS

**Files:** `src/lib/components/CommandPalette.svelte`.

- [ ] Modal triggered by `Cmd+K` / `Ctrl+K`. Input + result list.
- [ ] Two-mode: short queries (≤2 words) → quick-lookup against names/tags via `listEntities` cache; longer queries → `search(q)` (FTS) and render snippet HTML.
- [ ] Selecting a result navigates to its detail page.
- [ ] Commit: `feat(frontend): ⌘K command palette with quick + full-text search`.

---

## Task 13: Application kanban view

**Files:** `src/routes/application/+page.svelte` (special-case the generic list), `src/lib/components/{KanbanBoard,KanbanCard}.svelte`.

- [ ] When type is `application`, show a List/Kanban toggle in the top bar.
- [ ] `KanbanBoard`: 7 columns (planning, drafting, submitted, decision-pending, accepted, rejected, withdrawn). Each column shows applications matching its status.
- [ ] `KanbanCard`: program name (looked up via wikilink resolution from current cache), internal deadline, recent-emails count badge (from backlinks).
- [ ] Drag-and-drop between columns calls `updateEntity` with the new status. Use `svelte-dnd-action` (`npm install svelte-dnd-action`) for the DnD primitive.
- [ ] Commit: `feat(frontend): kanban view for Applications with drag-to-change-status`.

---

## Task 14: Dashboard

**Files:** `src/routes/+page.svelte`.

- [ ] Sections: Upcoming deadlines (next 30 days), Application pipeline (counts per status), Outstanding recommendations (Documents with `status: requested`), Recent emails (latest 10).
- [ ] All sections fetch directly via the API client; no caching beyond Svelte's standard reactivity.
- [ ] Commit: `feat(frontend): dashboard with deadlines + pipeline + outstanding recs`.

---

## Task 15: Deadlines view

**Files:** `src/routes/deadlines/+page.svelte`.

- [ ] Aggregate: walk all Applications + Programs to extract `deadline`/`internal_deadline`; walk all bodies (fetch detail for each) to extract `- [ ]` items with dates. Sort chronologically. Group by month.
- [ ] At v1 scale (≤200 entities) the cost is tolerable. If it gets slow later, the sidecar grows a `/deadlines` endpoint.
- [ ] Commit: `feat(frontend): unified deadlines view`.

---

## Task 16: Notes view

**Files:** `src/routes/notes/+page.svelte`.

- [ ] List entities of type `note` (which are stored in the notes folder by the sidecar — but Plan 1 surfaces them under the same `/entities/note` path? Verify: Plan 1's `_TYPE_TO_MODEL` includes `Note`, so they parse if frontmatter has the `eduport-type/note` tag. **Note** for the implementer: Plan 1 ships notes-as-entities, not notes-as-bare-files. The frontend treats them the same way as other entities; no separate handling needed for v1.)
- [ ] Display a flat list with title + first body line. Click → detail page.
- [ ] Commit: `feat(frontend): free-form notes list view`.

---

## Task 17: Trash view + restore endpoint

**Sidecar add:** `GET /trash` lists files in `.eduport-trash/`; `POST /trash/{name}/restore` restores. (Plan 1 deferred this; it's two small endpoints.)

**Files:** `sidecar/src/eduport/api/trash.py`, `sidecar/tests/test_api/test_trash_api.py`, `src/routes/trash/+page.svelte`, `src/lib/api/trash.ts`.

- [ ] **Sidecar side first**: add `/trash` GET and `/trash/{name}/restore` POST. Follow the Tasks 25-28 pattern from Plan 1. Tests under `test_api/test_trash_api.py`.
- [ ] **Frontend side**: `trash.ts` API client, route page lists trashed files with a "Restore" button each.
- [ ] Commit (one for sidecar, one for frontend): `feat(sidecar): /trash list + restore endpoints`, `feat(frontend): trash view`.

---

## Task 18: Empty state + first-run

**Files:** `src/routes/+layout.svelte` (loader checks settings).

- [ ] If `getSettings` returns 5xx (no settings yet — sidecar refused to start), the layout shows a "Welcome — pick a data folder" modal instead. The folder picker itself is a Plan 3 concern (native dialog); for now, accept a typed path and call `putSettings`.
- [ ] If settings present but data folder is empty (no entities of any type), show a friendly Dashboard with "Get started" tips.
- [ ] Commit: `feat(frontend): first-run + empty data folder UX`.

---

## Task 19: Status / errors panel

**Files:** `src/lib/components/ErrorBanner.svelte`, edit Sidebar.

- [ ] Status store polls `/health` and a new sidecar endpoint `GET /status` (returns parse-error count). Add the `/status` endpoint to the sidecar (small) following the Plan 1 pattern. (Or: query the existing `parse_errors` table via a simple SELECT-COUNT on each list call — acceptable for v1.)
- [ ] When parse errors exist, sidebar shows "Files with errors (N)" badge; clicking opens a modal listing them with "Open in Obsidian" stub buttons.
- [ ] When sidecar is down, top of layout shows a red banner "Eduport backend disconnected".
- [ ] Commit: `feat(frontend): status panel + error banner`.

---

## Task 20: Smoke test + ergonomics polish

- [ ] Run `npm run check` (svelte-check + tsc) — fix any real type errors.
- [ ] Run `npm run lint` — fix.
- [ ] Manually walk the golden path: dev server boots → list Programs → create one → edit it → toggle a checkbox → search for it → kanban view of Applications → drag a card → ⌘K palette → trash + restore.
- [ ] Commit: `chore(frontend): pass svelte-check + lint, smoke-tested`.

---

## Self-review (for the author of this plan)

**Spec coverage** vs design spec §7 (UI):
- §7.1 three-pane layout — Task 6.
- §7.2 hybrid editing model — Tasks 8 (BodyView), 9 (form), 10 (BodyEditor). "Open in Obsidian" stays a stub in Plan 2; Plan 3 wires the URI scheme.
- §7.3 cross-navigation + backlinks — Task 8 (clickable wikilinks, backlinks block).
- §7.4 document actions — partial in Task 8 (file metadata display + Open). The native dialogs (Reveal in file manager, Save copy) are Plan 3.
- §7.5 key views — covered: Dashboard (14), Programs/People/Univ/Lab/Document/Email lists (7), Applications kanban (13), Notes (16), Deadlines (15).
- §7.6 search — Task 12.
- §7.7 first-run / empty state — Task 18 (typed path, not native dialog).
- §7.8 soft-delete — Task 17 (Plan 1 already shipped the trash mechanism; this surfaces the UI + adds two small sidecar endpoints).
- §7.9 logging / error visibility — Task 19.

**Things deferred to Plan 3** (clearly out-of-scope here):
- Tauri native window, sidecar lifecycle from the shell, native file/save dialogs, "Reveal in file manager", `.eml` drag-and-drop hookup at the OS level (the in-page drop zone could work in Plan 2 via standard HTML5 DnD against the body).
- Packaging into `.app` / `.msi` / `AppImage`.

**Implementation pacing.** This plan is leaner than Plan 1 because (a) the contract with the sidecar is fixed and exercised by Plan 1's E2E test, (b) frontend work has fewer correctness traps, more visual judgment calls. Implementer: don't gold-plate visuals — match the spec's mockup (`docs/superpowers/specs/2026-05-06-eduport-design.md` §7.1) and ship.
