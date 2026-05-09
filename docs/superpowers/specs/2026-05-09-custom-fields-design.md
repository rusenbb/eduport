# Eduport — Custom Fields ("Properties") Design Spec

**Status:** draft for review
**Date:** 2026-05-09
**Author:** brainstormed with Claude
**Supersedes:** none — additive to [2026-05-06-eduport-design.md](2026-05-06-eduport-design.md)

---

## 1. Overview

This spec adds a **user-managed property system** to Eduport — Notion-style
custom fields that the user defines per entity type and that participate
fully in Eduport's existing UI surfaces (detail panel, sidebar, list view,
kanban, search).

The trigger use case is "tier" — the user wants to bucket Universities,
Programs, and Applications into a tier (reach / target / safety). Rather
than hardcoding a `tier` field on three entity models, this spec generalises
the mechanism: any entity type gains a list of user-defined properties, with
typed values, persistent across sync, fully integrated into the existing UI.

Once shipped, "tier" is implemented by the user clicking **Settings →
Schema → University → Add tier field** (a one-click template).

## 2. Goals and non-goals

### Goals

- **Per-entity-type custom properties.** Each of the 8 entity types has its
  own independent property list. A property defined on University does not
  exist on Application unless the user adds it there too.
- **Typed properties.** Eight property types: `text`, `number`, `date`,
  `checkbox`, `single-select`, `multi-select`, `url`, `relation` (a wikilink
  to another Eduport entity, with optional target-type restriction).
- **Files-are-truth, sync-safe.** Property *definitions* live in the data
  folder (`.eduport/schema.yaml`) and sync alongside entity files. Property
  *values* live in the same flat YAML frontmatter as built-in fields.
- **Obsidian-friendly.** Custom values appear flat in YAML (`tier: reach`),
  not under a `custom:` namespace. They look like any other YAML key when
  edited in Obsidian.
- **Lenient validation.** Built-in fields validate strictly. Custom-field
  violations (wrong type, out-of-options select, orphaned key) generate UI
  warning chips but never reject the entity — files always load.
- **Full UI integration.** Custom properties appear in: detail panel
  (always), sidebar chips (selects), list view filter / sort / columns,
  kanban group-by, FTS5 search (text/url types).

### Non-goals

- **Modifying built-in fields.** `Application.status`, `University.country`,
  etc. remain Pydantic-defined and immutable. The schema editor surfaces
  them read-only for context but cannot rename or remove them.
- **Formula and rollup property types.** A formula engine is a separate,
  larger project.
- **Multi-user schema, schema permissions, schema history.** Single-user
  app — user owns the schema absolutely. Schema is plain YAML in the data
  folder; the user's filesystem and sync layer are the audit trail.
- **Backwards-compat for not-yet-existing data.** Eduport is pre-1.0; no
  migration support for users who edited entity files outside the app
  before the schema file existed (there is no such cohort).

## 3. Schema file

### 3.1 Location and layout

A single file at `<data folder>/.eduport/schema.yaml`. The dotfile prefix
keeps it tidy; the directory is reserved for future Eduport-internal files.

```yaml
version: 1
types:
  university:
    properties:
      - key: tier
        name: Tier
        type: single-select
        description: Reach / target / safety bucket
        required: false
        options:
          - {value: reach,  label: Reach,  color: red}
          - {value: target, label: Target, color: yellow}
          - {value: safety, label: Safety, color: green}
      - key: gpa_required
        name: GPA Required
        type: number
  application:
    properties:
      - key: priority
        name: Priority
        type: single-select
        options:
          - {value: high, label: High, color: red}
          - {value: low,  label: Low,  color: gray}
  lab: { properties: [] }
  person: { properties: [] }
  program: { properties: [] }
  document: { properties: [] }
  email: { properties: [] }
  note: { properties: [] }
```

All eight entity types must be present (with possibly empty `properties`
arrays). The schema file's own structure is strict (`extra="forbid"` on the
schema-side Pydantic models) — it is app-managed; the user edits it through
the Schema editor, not by hand. (Hand-editing remains possible and supported,
but is not the primary affordance.)

### 3.2 Property definition shape

Common fields:

| Field | Required | Notes |
|---|---|---|
| `key` | yes | Slugified, `[a-z0-9_]+`. Used as the YAML key. Immutable after creation (renaming would orphan all values). |
| `name` | yes | Display label. Free-form, can be edited at any time. |
| `type` | yes | One of the eight property types. Immutable after creation. |
| `description` | no | Free-form helper text shown in tooltip. |
| `required` | no | Default `false`. If `true`, entity creation/save UI flags missing values; lenient validator generates a "missing required" warning chip but does not reject the file. |
| `default` | no | Type-appropriate default value, applied when creating new entities. |

Type-specific fields:

| Type | Extra fields | YAML value shape | Notes |
|---|---|---|---|
| `text` | — | string | |
| `number` | `unit?` (display-only string) | int or float | |
| `date` | — | ISO date `YYYY-MM-DD` | |
| `checkbox` | — | bool | |
| `single-select` | `options: [{value, label, color}]` | string matching an option `value` | Color is one of a fixed palette (gray/red/orange/yellow/green/teal/blue/purple/pink). |
| `multi-select` | `options: [{value, label, color}]` | list of option `value`s | |
| `url` | — | string | Validated as `HttpUrl`. |
| `relation` | `target_types?: [entity_type]` | wikilink string `[[target-id]]` | Empty `target_types` → any entity type. |

### 3.3 Schema validation rules

Enforced by `SchemaStore` on every schema save:

1. **Key collision with built-ins.** A custom `key` may not equal any
   Pydantic field name on the corresponding entity model. The `BaseEntity`
   reserved keys (`tags`, `name`) are blocked on every type. Per-type
   reserved keys come from the model's `model_fields`.
2. **Key collision with other custom fields on the same type.** Within a
   type, `key` is unique.
3. **Type immutability.** Once a property exists, its `type` cannot change.
   To "change type", the user deletes the property (orphaning values) and
   creates a new one.
4. **Key immutability.** `key` cannot be edited after creation. `name` can.
5. **Option `value` immutability.** Once a select option is added, its
   `value` cannot be edited (would orphan values referencing it). Its
   `label` and `color` are editable. Deleting an option orphans values
   referencing it (lenient: warning chip on entity).
6. **Relation target validity.** `target_types` entries must be members
   of `EntityType`.

## 4. Entity model changes

### 4.1 Pydantic relaxation

Each entity model in `sidecar/src/eduport/models/` (University, Lab, Person,
Program, Application, Document, Email, Note) changes from
`extra="forbid"` to `extra="allow"`. This is necessary: Pydantic must accept
arbitrary custom keys that pass the schema-driven validator.

To preserve the original strictness for *unknown* keys, a new
post-validation step runs after Pydantic's own:

```python
# in sidecar/src/eduport/parsers/entity.py
def validate_custom_fields(entity: BaseEntity, schema: Schema) -> list[ValueWarning]:
    """Validate custom keys against the loaded schema. Returns warnings;
    does not raise. Built-in keys are not checked here (Pydantic handled them).
    """
    type_schema = schema.for_type(entity.entity_type())
    declared_keys = {p.key for p in type_schema.properties}
    builtin_keys = set(entity.model_fields)
    warnings: list[ValueWarning] = []

    extra_keys = set(entity.model_extra) if entity.model_extra else set()
    for key in extra_keys:
        if key not in declared_keys:
            warnings.append(ValueWarning(key=key, kind="orphaned"))
            continue
        prop = type_schema.property(key)
        value = entity.model_extra[key]
        warnings.extend(_check_value(prop, value))
    return warnings
```

Built-in keys remain governed by Pydantic's normal type checks. Built-in
violations remain *strict* — they go to the parse-error list as today.
Custom violations are *lenient* — they become `ValueWarning`s on the entity
payload returned to the frontend.

### 4.2 Per-property value validation

A `_check_value(prop, value)` helper validates each custom value against
its property definition:

- `text`: any string. (No warnings possible.)
- `number`: must be int or float. Otherwise `kind="type_mismatch"`.
- `date`: must parse as ISO date. Otherwise `kind="type_mismatch"`.
- `checkbox`: must be bool. Otherwise `kind="type_mismatch"`.
- `single-select`: must be one of the option values. Otherwise
  `kind="out_of_options"` with the offending value attached.
- `multi-select`: must be a list of strings; each must be an option value.
- `url`: must be a string parseable as `HttpUrl`. Otherwise
  `kind="type_mismatch"`.
- `relation`: must be a wikilink to a known entity. If `target_types` is
  set, the target's entity type must be in it. Broken-link →
  `kind="broken_link"`; wrong-type → `kind="wrong_target_type"`.

Required-but-missing yields `kind="required_missing"`.

### 4.3 Entity payload shape (API)

Entity GET responses gain a `value_warnings` field:

```json
{
  "id": "K9p3",
  "name": "ETH Zurich",
  "type": "university",
  "frontmatter": { "country": "Switzerland", "tier": "foo", ... },
  "body": "...",
  "value_warnings": [
    {"key": "tier", "kind": "out_of_options", "value": "foo"}
  ]
}
```

The frontend reads `value_warnings` to show warning chips inline next to
the offending field in the detail panel and a count badge in the list view.

## 5. SchemaStore (sidecar)

A new module `sidecar/src/eduport/store/schema_store.py`:

- `SchemaStore.load() -> Schema` — reads `.eduport/schema.yaml`. If the file
  is missing, returns an empty schema and writes the seed (eight types,
  empty property lists). Cached in memory; invalidated by `reload()`.
- `SchemaStore.current() -> Schema` — current cached schema.
- `SchemaStore.update(...) -> Schema` — applies a schema mutation, runs
  validation rules (§3.3), writes atomically (temp file + rename), bumps
  the in-memory cache, and triggers re-validation of all entities of the
  affected type. Returns the new schema.
- `SchemaStore.is_builtin_key(entity_type, key) -> bool` — used by collision
  checks and the schema-editor frontend.

The watcher (`sidecar/src/eduport/watcher.py`) watches `.eduport/schema.yaml`
in addition to entity files. On external edits (e.g. user hand-edits the
schema file), it triggers `SchemaStore.reload()` and re-validates entities.

## 6. API surface

New endpoints under `/api/schema`:

| Method | Path | Body / response | Purpose |
|---|---|---|---|
| `GET` | `/api/schema` | full schema | Read whole schema (used by schema editor + frontend bootstrap). |
| `GET` | `/api/schema/types/{type}` | type schema | Read one type's properties. |
| `POST` | `/api/schema/types/{type}/properties` | property body | Add a property. |
| `PATCH` | `/api/schema/types/{type}/properties/{key}` | partial body | Edit `name`, `description`, `required`, `default`, `options[].label`, `options[].color`, `unit`. (Type, key, option values are immutable — see §3.3.) |
| `DELETE` | `/api/schema/types/{type}/properties/{key}` | — | Remove a property. Orphans values; entity files are not rewritten. |
| `POST` | `/api/schema/types/{type}/properties/{key}/purge_orphans` | — | Rewrite all entity files of this type to remove the orphaned key. Irreversible (subject to soft-delete trash semantics). |
| `POST` | `/api/schema/templates/tier` | `{ types: [type, ...] }` | One-click template that adds the standard tier single-select to the listed types. Pure shortcut over `POST .../properties`. |

Existing entity endpoints gain `value_warnings` in their response payloads
(no breaking changes — additive field).

## 7. SQLite indexer changes

`sidecar/src/eduport/index/`:

### 7.1 New `properties` table

```sql
CREATE TABLE properties (
    entity_id TEXT NOT NULL,
    key       TEXT NOT NULL,
    type      TEXT NOT NULL,            -- mirrors property type for query convenience
    value_text TEXT,                    -- text, url, single-select, relation target
    value_num  REAL,                    -- number, checkbox-as-0/1, date-as-julian
    value_date TEXT,                    -- ISO date duplicate for direct comparison
    value_multi TEXT,                   -- JSON array for multi-select
    PRIMARY KEY (entity_id, key)
);
CREATE INDEX idx_properties_key_text ON properties(key, value_text);
CREATE INDEX idx_properties_key_num  ON properties(key, value_num);
CREATE INDEX idx_properties_key_date ON properties(key, value_date);
```

Filter and sort queries hit this table. Single-select / relation values
(both shaped as `value_text`) get the same index treatment.

### 7.2 FTS5 extension

Existing FTS5 virtual table grows a column `custom_text` populated with
the concatenated text+url custom values for each entity. Schema version
bumps from N to N+1 in `index/schema.py`; on first load after upgrade, the
indexer drops and rebuilds (`reconcile.py` already handles full rebuilds
for stale indexes).

### 7.3 Reconciliation

When the schema changes (property added/removed/edited), the indexer
re-derives the affected rows. Adding a property: re-scan all entities of
that type, insert any present values. Removing a property: delete rows
matching `(key=...)`. Editing select options: no rewrite needed
(`value_text` is the option `value` string, immutable).

## 8. Frontend

### 8.1 New components

In `frontend/src/lib/properties/`:

- `PropertyValue<type>.svelte` — read-only renderer, one per type. Used
  in detail panel, list-view custom columns, sidebar.
- `PropertyEditor<type>.svelte` — edit-mode component, one per type. Used
  in detail panel inline editing, schema editor's "default value" picker,
  filter bar's value picker.
- `PropertyTypeIcon.svelte` — type-indicating icon (used in schema editor
  + filter bar dropdowns).
- `PropertyWarningChip.svelte` — renders a `value_warning` inline.

### 8.2 Schema editor

New route: `frontend/src/routes/settings/schema/+page.svelte`.

Layout: vertical tabs (one per entity type) → list of properties for the
selected type → drag-handle to reorder → "+ Add property" button → "Add
tier" template button (visible on University, Program, Application).

Per-property row: type icon + name + key (small) + type badge + edit/delete
buttons. Edit dialog is type-specific (e.g. select shows option list with
add/remove/reorder; relation shows target-types multi-picker).

Built-in fields are listed above the custom list, read-only, greyed.

### 8.3 List-view filter and sort

Above each entity-type list view, a property filter bar:

- "Add filter" dropdown shows all properties (built-in and custom) on this
  type.
- Adding a filter creates a chip with type-appropriate UI:
  - select / multi-select → multi-pick from options
  - number → min / max range
  - date → from / to date pickers
  - text / url → contains-string input
  - checkbox → tri-state (any / true / false)
  - relation → target-entity picker
- Filters AND together. Active-filter chips visible above the list.
- Sort dropdown lists all sortable properties (everything except relation).

Filter and sort state is URL-synced (matches the existing pattern from the
[group-by-application toggle](frontend/src/routes)).

### 8.4 List-view custom columns

A "Properties" toggle dropdown at the top of each list view lets the user
show/hide custom properties as columns. Default: all hidden. The selected
column set is persisted per entity type in localStorage (per-machine UI
preference, not synced).

### 8.5 Kanban group-by

The kanban view (Application) gets a "Group by" dropdown at the top.
Default: `status` (preserves existing behavior). Other options: any
single-select property defined on Application. Drag-drop on a non-status
group-by writes the property value (e.g. drag a card from `Tier: target`
to `Tier: reach` patches `tier`).

### 8.6 Sidebar

Existing sidebar (per entity type) currently shows tag chips with counts.
After this spec, it gains property-aggregation sections under each entity
type:

- For each `single-select` property: a chip group, one chip per option
  value, with counts and the property's color palette.
- For each `multi-select` property: same.
- For each `checkbox` property: two chips ☑ / ☐ with counts.
- Date / number / text / url / relation properties do not appear in the
  sidebar (filter bar is the right place for those).

Clicking a property chip applies the corresponding filter to the list view.

### 8.7 Detail panel

Built-in fields render above (existing layout). Below them, a "Properties"
section iterates the schema's property list in order. Each row is a
`PropertyValue<type>` in display mode; clicking switches to
`PropertyEditor<type>`. Inline value warnings render via
`PropertyWarningChip` next to the offending row.

A "+ Add property" button at the bottom of the Properties section
deep-links to **Settings → Schema → {this entity type}** with focus on
the "+ Add property" button (it does not let you add a property without
also defining it in the schema — there are no per-entity ad-hoc keys).

### 8.8 Command palette / search

⌘K already searches via FTS5 — once the FTS5 column is added (§7.2), it
picks up custom text/url values automatically. No frontend changes
beyond the FTS5 schema bump. Sidebar / palette result rendering already
shows entity name + type + snippet; matched custom values fall under the
existing snippet rendering.

## 9. Data flow examples

### 9.1 User adds tier to University

1. User opens **Settings → Schema → University**, clicks **Add tier**.
2. Frontend `POST /api/schema/templates/tier { types: ["university"] }`.
3. Sidecar's `SchemaStore.update(...)` validates (no key collision, valid
   type config), writes the new property to `.eduport/schema.yaml`, bumps
   in-memory cache, rebuilds the SQLite `properties` rows for University
   (no rows yet — no values).
4. Sidecar broadcasts a `schema_changed` event over the existing watcher
   notification channel.
5. Frontend re-fetches schema; sidebar gains a "Tier" chip group (with no
   values yet); detail panels for Universities show a "Tier" property row
   (empty); kanban-by-tier becomes available (though only on Application,
   so no effect here unless the user adds tier there too).

### 9.2 User sets tier on a university

1. User opens ETH Zurich detail panel, clicks the empty "Tier" row.
2. `PropertyEditor<single-select>` opens with the three options.
3. User picks "reach". Frontend `PATCH /api/entities/eth-zurich-K9p3 { tier: "reach" }`.
4. Sidecar applies the patch via the existing entity-update path. The
   custom field validator confirms `reach` is a valid option, no warnings.
5. Atomic write rewrites `eth-zurich-K9p3.md` with the new YAML key.
6. Indexer upserts a row in `properties (entity_id="K9p3", key="tier",
   type="single-select", value_text="reach")`.
7. Sidebar's "Tier: Reach" chip count increments.

### 9.3 User edits the YAML in Obsidian, types `tier: foo`

1. User saves the file in Obsidian. OS triggers watcher event.
2. Sidecar parses the file. Built-ins validate fine. Custom validator:
   `tier=foo` is not in `[reach, target, safety]` → `value_warnings:
   [{key: "tier", kind: "out_of_options", value: "foo"}]`.
3. The entity loads. Indexer stores `value_text="foo"` regardless (lenient).
4. Frontend re-fetches the entity; renders the value with a warning chip
   next to it ("'foo' is not in options"). The list view's badge increments.
5. User opens the detail panel and either picks a valid option or edits
   the schema to add "foo" as an option.

### 9.4 User deletes the tier property

1. User opens **Settings → Schema → University**, clicks delete on tier.
2. Confirm dialog: "Tier has values on 14 universities. Deleting the
   property orphans those values (they remain in your YAML files but
   become invisible in the app). You can re-add the property later to
   restore them, or run 'Purge orphans' to permanently remove the YAML
   keys."
3. User confirms. `DELETE /api/schema/types/university/properties/tier`.
4. SchemaStore removes the property from the schema file. Indexer drops
   `properties` rows for `key="tier"`.
5. Entity files are NOT rewritten. The 14 universities still have
   `tier: reach` in their YAML. The custom validator now classifies these
   as `kind="orphaned"`, surfaced as a tally ("14 orphaned `tier` values")
   in the schema editor with a "Purge" button.

## 10. Migration / first-run

- On sidecar startup, if `.eduport/schema.yaml` doesn't exist, `SchemaStore`
  writes a seed file (eight types, empty `properties` arrays). This is the
  *only* migration step; no entity files need to change.
- On schema-version bump (currently `1`), the FTS5 schema rebuild logic
  in `index/reconcile.py` triggers a full reindex (it already handles
  this case for index-version bumps).
- Pre-existing entity files that were valid under `extra="forbid"` remain
  valid under `extra="allow"` + custom validation (any keys present that
  weren't built-in would have failed `forbid` and so don't exist).

## 11. Testing strategy

### Sidecar (pytest)

- **`test_schema_store.py`**: load/save round-trip, atomic write, seed
  creation, watcher-triggered reload, mutation API (add / edit / delete /
  template), all validation rules from §3.3 (collisions, immutability of
  key/type/option-value).
- **`test_entity_validation.py`**: each property type's `_check_value`
  branch (valid + every warning kind), built-ins still strict, custom
  fields lenient.
- **`test_schema_api.py`**: every endpoint, error responses (collision,
  unknown type, missing fields), template endpoint shape.
- **`test_index_properties.py`**: properties-table writes, FTS5 column
  population, re-derivation on schema changes.

### Frontend (`npm run check` for types; component tests where they exist)

- Manual / visual: schema editor flows, detail-panel inline edits, filter
  bar interactions per type, kanban group-by switching, sidebar chip
  counts, warning-chip rendering for each warning kind.

### End-to-end (manual + scripted curl in dev)

- Add tier to University via template → set value via PATCH → edit YAML
  externally to introduce a violation → confirm warning surfaces → purge
  orphans → confirm cleanup.

## 12. Open questions deferred to implementation

- Exact filter-chip UX for `relation` (autocomplete vs modal picker) —
  decide during frontend build, refer to existing wikilink-picker affordances.
- Color palette for select options — pick a 9-color set during design pass;
  not architecturally load-bearing.
- Whether sidebar property chips collapse by default for entity types with
  many properties — UX polish, decide during frontend build.

## 13. Out of scope (future work)

- Formula and rollup property types.
- Cross-type "global" properties (the spec is per-type only).
- Schema versioning UI / undo for schema mutations beyond standard git/sync
  recovery.
- Bulk-edit operations on property values across many entities at once
  (e.g. "set all unset tiers to safety"). Possible follow-up.
- Importing schema from another data folder.

---

**End of spec.**
