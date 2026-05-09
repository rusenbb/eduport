# Eduport — "Make it feel like Notion" execution plan

**Status:** in progress
**Date:** 2026-05-09
**Goal:** close the experience gap between Eduport's substance (we have it) and Notion's feel (we don't yet).

This plan is executed end-to-end in one session. Every phase ends with:

1. Tests passing (sidecar pytest + ruff, frontend `svelte-check`).
2. A standalone, conventional commit.

If a phase breaks, it doesn't get committed; the previous green state stays.

## Phases

### Phase 0 — baseline commit (foundation already built)

Commit the existing custom-fields work (schema editor, properties table, FTS5
custom column, watcher schema integration, etc.). Single feature commit; sets
a clean reference point for everything that follows.

### Phase 1 — sidecar: saved-views storage + API

New file `<data>/.eduport/views.yaml` holds named views per entity type.
Each view captures: `kind` (table | list | board), `entity_type`, `name`,
`filter` (text/num/date), `sort`, `group_by?`, `columns?` (which custom
properties to show in table view), `card_properties?` (which to render on
kanban cards).

- `models/view.py` — Pydantic models, strict, sibling of `models/schema.py`.
- `store/view_store.py` — load/seed/atomic save (mirrors `SchemaStore`).
- `api/views_api.py` — `GET /api/views`, `GET /api/views/{type}`,
  `POST /api/views/{type}` (create), `PATCH /api/views/{type}/{id}`,
  `DELETE /api/views/{type}/{id}`.
- Watcher emits `views_modified` for external edits (analogous to
  `schema_modified`).
- Tests for models, store, API.

### Phase 2 — frontend: editable table view

The single biggest "feels like Notion" lever.

- `lib/components/TableView.svelte` — header row of columns, body row per
  entity. Click any cell → swap to inline `PropertyEditor`; blur or Enter
  to save, Escape to cancel.
- `lib/components/properties/PropertyTypeIcon.svelte` — 8 small icons (one
  per property type) used in headers and elsewhere.
- View mode toggle on every entity type: List ↔ Table (URL-synced via
  `?view=table`).
- Column visibility: dropdown above the table to show/hide each property.
  Persisted per entity-type in localStorage.
- The table also surfaces built-in fields the spec calls out (e.g. `country`
  for University, `status` for Application) as fixed leading columns.

### Phase 3 — group-by + property icons everywhere

- Group-by-select picker for the table/list view: collapsible sections,
  one per option value, plus an "Uncategorized" bucket. Reuses the kanban
  `groupBy` data shape.
- Property type icons applied in: schema editor list, filter bar, sidebar
  chips header, table headers.
- Inline `+ Add property` composer on the detail panel — opens a small
  dialog scoped to the current entity type, doesn't navigate away.

### Phase 4 — saved views frontend

- `lib/api/views.ts` + `lib/stores/views.ts`.
- `lib/components/ViewTabs.svelte` — tabs across the top of every entity
  list view, one per saved view + a "+ New view" button + a default "All"
  view.
- `lib/components/SaveViewDialog.svelte` — captures current filter/sort/
  group/columns/view-mode into a named view.
- View management: rename, duplicate, delete via a small per-tab menu.

### Phase 5 — drag-reorder polish + kanban customization

- HTML5 dragstart-based reordering for properties in the schema editor.
- Same for select options inside the property config dialog.
- Kanban "Card properties" dropdown — pick which custom properties show
  on each card (label-style chips for selects, raw value for text/number/
  date). Stored per saved view; default = none beyond name.

### Phase 6 — final integration, tests, build

- Full sidecar `pytest -q` + `ruff check`.
- Frontend `npm run check` clean (0 errors, ideally 0 warnings).
- Run `python3 scripts/build_desktop.py` to produce the new `.deb` and
  `.rpm`. The user runs `sudo apt install ./...deb` themselves on return.
- Final commit + summary line in the README's "Highlights" section.

## Decisions made up-front (no further user pings)

- **Saved views are sync-friendly.** They live in `.eduport/views.yaml`
  alongside `schema.yaml`. Same atomic-write semantics, same watcher
  integration. Cross-machine consistency matches the schema's pitch.
- **Column visibility is per-machine** (localStorage). It's a UI preference,
  not a data definition; lives in `eduport:columns:<entity_type>`.
- **"Group by" is a view-level setting.** Both the kanban's group-by and
  the table's group-by save into the same `group_by` slot when a view is
  saved. One mental model.
- **No formulas, rollups, calendar / gallery / timeline views.** Out of
  scope per the prior conversation.
- **Auto-managed `created_at` / `edited_at` are deferred.** Useful but
  invisible until you sort by them; not in the first impression.

## Risks I'm watching

- **Inline-editable table cells** are fiddly: focus management, keyboard
  shortcuts, save-on-blur-vs-cancel. I'll model after the existing detail-
  panel inline edit (which is already proven).
- **Drag-reorder** in browsers is even more fiddly. I'll use the native
  HTML5 drag API and accept that it's slightly less polished than a
  dedicated library — adding a drag library (`@thisux/sveltednd` or similar)
  is a sub-project I'd rather not introduce.
- **Watcher race** on `views.yaml` external edits: same shape as schema —
  reload on event, drop in-memory cache. Solved.

## Out of scope (explicit)

- Calendar / gallery / timeline view kinds.
- Rollup / formula property types.
- Multi-sort and complex filter expressions (AND-only stays).
- Auto-managed timestamps.
- Drag-to-reorder *entity rows* (Notion lets you manually order; we'd need
  a position column. Future work.).
- Database templates ("New from template" — saved views deliver most of
  this need by capturing default values).

---

End of plan. Phases 0–6 below mark progress as commits land.

- [ ] Phase 0 — baseline commit
- [ ] Phase 1 — saved-views storage + API
- [ ] Phase 2 — editable table view
- [ ] Phase 3 — group-by + property icons + inline `+ Add property`
- [ ] Phase 4 — saved views frontend
- [ ] Phase 5 — drag-reorder polish + kanban card customization
- [ ] Phase 6 — final tests + build
