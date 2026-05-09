<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { getContext } from 'svelte';
	import type { Writable } from 'svelte/store';
	import { deleteEntity, getEntity, listEntities } from '$lib/api/entities';
	import {
		filterEntitiesByProperties,
		hasActiveFilters,
		parseFilterParams,
		writeFilterParams
	} from '$lib/api/schema';
	import { filters } from '$lib/stores/filters';
	import { schemaStore } from '$lib/stores/schema';
	import { settings } from '$lib/stores/settings';
	import type { EntityDetail, EntityListItem, EntityType } from '$lib/types';
	import { DEFAULT_PROPERTY_FILTERS, type PropertyFilters } from '$lib/types/schema';
	import { TYPE_LABELS } from '$lib/entities/meta';
	import { confirmDestructive } from '$lib/tauri';
	import EntityBodyEditor from './EntityBodyEditor.svelte';
	import DetailPanel from './DetailPanel.svelte';
	import EntityForm from './EntityForm.svelte';
	import EntityList from './EntityList.svelte';
	import EmailGroupedList from './EmailGroupedList.svelte';
	import KanbanBoard from './KanbanBoard.svelte';
	import TableView from './TableView.svelte';
	import ColumnVisibilityMenu from './properties/ColumnVisibilityMenu.svelte';
	import PropertyFilterBar from './properties/PropertyFilterBar.svelte';
	import { FIELD_DEFS } from '$lib/entities/meta';

	let { type, fileId = null }: { type: EntityType; fileId?: string | null } = $props();

	let items: EntityListItem[] = $state([]);
	let details: Record<string, EntityDetail | null> = $state({});
	let selected: EntityDetail | null = $state(null);
	let loading = $state(true);
	let detailLoading = $state(false);
	let error: string | null = $state(null);
	let creating = $state(false);
	let editingForm = $state(false);
	let editingBody = $state(false);
	let focusMode = $state(false);
	let propertyFilters: PropertyFilters = $state(parseFilterParams(page.url.searchParams));

	function syncFiltersToUrl(next: PropertyFilters) {
		propertyFilters = next;
		const url = new URL(page.url);
		writeFilterParams(url.searchParams, next);
		void goto(url, { replaceState: true, keepFocus: true, noScroll: true });
	}
	type ViewMode = 'list' | 'table' | 'kanban';
	const view: ViewMode = $derived.by(() => {
		const v = page.url.searchParams.get('view');
		if (v === 'kanban' && type === 'application') return 'kanban';
		if (v === 'table') return 'table';
		return 'list';
	});
	const groupBy = $derived(page.url.searchParams.get('group') === 'application' ? 'application' : 'none');
	const customProperties = $derived($schemaStore.schema?.types[type]?.properties ?? []);

	// Single-select properties on Application that the user can group the kanban by.
	const kanbanGroupableProps = $derived(
		type === 'application'
			? customProperties.filter((p) => p.type === 'single-select')
			: []
	);
	const kanbanGroupKey = $derived(page.url.searchParams.get('kanban_by') ?? 'status');
	const kanbanGroupBy = $derived.by(() => {
		if (kanbanGroupKey === 'status') return undefined; // built-in path
		const prop = kanbanGroupableProps.find((p) => p.key === kanbanGroupKey);
		if (!prop || prop.type !== 'single-select') return undefined;
		return {
			key: prop.key,
			columns: prop.options.map((o) => ({ value: o.value, label: o.label, color: o.color }))
		};
	});

	function setKanbanGroup(key: string) {
		const url = new URL(page.url);
		if (key === 'status') url.searchParams.delete('kanban_by');
		else url.searchParams.set('kanban_by', key);
		void goto(url, { replaceState: true, keepFocus: true, noScroll: true });
	}

	function setView(next: ViewMode) {
		const url = new URL(page.url);
		if (next === 'list') url.searchParams.delete('view');
		else url.searchParams.set('view', next);
		void goto(url, { replaceState: true, keepFocus: true, noScroll: true });
	}

	// Column visibility (table view) — persisted per entity type in
	// localStorage. Defaults to "all custom properties, no built-ins" so the
	// table starts empty-but-functional and the user opts in to the noise.
	const COLUMNS_STORAGE_PREFIX = 'eduport:columns:';

	function loadColumns(t: EntityType): { custom: string[]; builtin: string[] } | null {
		if (typeof window === 'undefined') return null;
		try {
			const raw = window.localStorage.getItem(COLUMNS_STORAGE_PREFIX + t);
			if (!raw) return null;
			const parsed = JSON.parse(raw);
			if (typeof parsed === 'object' && Array.isArray(parsed.custom) && Array.isArray(parsed.builtin)) {
				return { custom: parsed.custom, builtin: parsed.builtin };
			}
		} catch {
			/* ignore */
		}
		return null;
	}

	function saveColumns(t: EntityType, value: { custom: string[]; builtin: string[] }) {
		if (typeof window === 'undefined') return;
		try {
			window.localStorage.setItem(COLUMNS_STORAGE_PREFIX + t, JSON.stringify(value));
		} catch {
			/* localStorage is best-effort */
		}
	}

	let visibleCustomKeys: string[] = $state([]);
	let visibleBuiltinKeys: string[] = $state([]);

	$effect(() => {
		// Whenever entity type changes, restore columns from localStorage,
		// or seed sensible defaults: all custom properties, no built-ins.
		const t = type;
		const stored = loadColumns(t);
		if (stored) {
			visibleCustomKeys = stored.custom;
			visibleBuiltinKeys = stored.builtin;
		} else {
			visibleCustomKeys = customProperties.map((p) => p.key);
			visibleBuiltinKeys = [];
		}
	});

	function persistColumns(next: { custom: string[]; builtin: string[] }) {
		visibleCustomKeys = next.custom;
		visibleBuiltinKeys = next.builtin;
		saveColumns(type, next);
	}

	function setGroupBy(next: 'none' | 'application') {
		const url = new URL(page.url);
		if (next === 'none') url.searchParams.delete('group');
		else url.searchParams.set('group', next);
		void goto(url, { replaceState: true, keepFocus: true, noScroll: true });
	}

	const newAction = getContext<Writable<{ label: string; onClick: () => void } | null>>('eduport:newAction');
	const selectedFileId = $derived(fileId ?? undefined);

	async function loadList() {
		loading = true;
		error = null;
		try {
			// Property filters / sort use the indexed query path; tag filters use
			// the existing list endpoint. When both apply, fetch via property
			// filters and intersect against the tag-list result.
			let baseItems: EntityListItem[];
			if (hasActiveFilters(propertyFilters)) {
				baseItems = await filterEntitiesByProperties(type, propertyFilters);
				if ($filters.tags.length > 0) {
					const tagged = new Set(
						(await listEntities(type, $filters.tags)).map((i) => i.file_id)
					);
					baseItems = baseItems.filter((item) => tagged.has(item.file_id));
				}
			} else {
				baseItems = await listEntities(type, $filters.tags);
			}
			items = baseItems;
			const nextDetails: Record<string, EntityDetail | null> = {};
			await Promise.all(
				items.map(async (item) => {
					try {
						nextDetails[item.file_id] = await getEntity(type, item.file_id);
					} catch {
						nextDetails[item.file_id] = null;
					}
				})
			);
			details = nextDetails;
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			loading = false;
		}
	}

	async function loadDetail() {
		if (!fileId) {
			selected = null;
			return;
		}
		detailLoading = true;
		try {
			selected = await getEntity(type, fileId);
			details = { ...details, [fileId]: selected };
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
			selected = null;
		} finally {
			detailLoading = false;
		}
	}

	$effect(() => {
		const _filterKey =
			$filters.tags.join('\u0000') + '|' + JSON.stringify(propertyFilters);
		newAction?.set({ label: `New ${TYPE_LABELS[type]}`, onClick: () => (creating = true) });
		void loadList();
		return () => newAction?.set(null);
	});

	$effect(() => {
		// Re-parse filters from the URL whenever the entity type or query
		// string changes — keeps sidebar-driven navigation in sync with the
		// filter bar's local state.
		type;
		propertyFilters = parseFilterParams(page.url.searchParams);
	});

	$effect(() => {
		void schemaStore.load();
	});

	$effect(() => {
		void loadDetail();
	});

	async function handleDelete() {
		if (!selected || !fileId) return;
		if (($settings?.confirm_deletes ?? true) && !(await confirmDestructive(`Move "${selected.entity.name}" to trash?`))) return;
		try {
			await deleteEntity(type, fileId);
			selected = null;
			focusMode = false;
			await loadList();
			goto(`/${type}`);
		} catch (e) {
			alert(`Delete failed: ${e instanceof Error ? e.message : String(e)}`);
		}
	}
</script>

<div class="grid h-full min-h-0 grid-cols-[minmax(360px,1fr)_minmax(360px,440px)] overflow-hidden">
	<section class="flex min-w-0 flex-col overflow-hidden border-r border-[var(--color-border)]">
		<header class="flex items-center justify-between border-b border-[var(--color-border)] px-4 py-2">
			<div>
				<h1 class="text-sm font-semibold">{TYPE_LABELS[type]}</h1>
				<p class="text-[10px] uppercase tracking-wider text-[var(--color-muted)]">{items.length} items</p>
			</div>
			<div class="flex items-center gap-2">
				{#if view === 'table'}
					<ColumnVisibilityMenu
						properties={customProperties}
						builtinFields={FIELD_DEFS[type] ?? []}
						{visibleCustomKeys}
						{visibleBuiltinKeys}
						onChange={persistColumns}
					/>
				{/if}
				{#if type === 'application' && view === 'kanban' && kanbanGroupableProps.length > 0}
					<label class="flex items-center gap-1 text-xs">
						<span class="text-[var(--color-muted)]">Group by</span>
						<select
							value={kanbanGroupKey}
							onchange={(e) => setKanbanGroup((e.currentTarget as HTMLSelectElement).value)}
							class="rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-2 py-0.5 text-xs"
						>
							<option value="status">Status (built-in)</option>
							{#each kanbanGroupableProps as p}
								<option value={p.key}>{p.name}</option>
							{/each}
						</select>
					</label>
				{/if}
				{#if type === 'email'}
					<div class="flex rounded border border-[var(--color-border)] p-0.5 text-xs">
						<button class="rounded px-2 py-1" class:active={groupBy === 'none'} onclick={() => setGroupBy('none')}>Chronological</button>
						<button class="rounded px-2 py-1" class:active={groupBy === 'application'} onclick={() => setGroupBy('application')}>By app</button>
					</div>
				{/if}
				<div class="flex rounded border border-[var(--color-border)] p-0.5 text-xs">
					<button class="rounded px-2 py-1" class:active={view === 'list'} onclick={() => setView('list')}>List</button>
					<button class="rounded px-2 py-1" class:active={view === 'table'} onclick={() => setView('table')}>Table</button>
					{#if type === 'application'}
						<button class="rounded px-2 py-1" class:active={view === 'kanban'} onclick={() => setView('kanban')}>Kanban</button>
					{/if}
				</div>
			</div>
		</header>

		{#if customProperties.length > 0 && !(type === 'application' && view === 'kanban')}
			<PropertyFilterBar
				properties={customProperties}
				filters={propertyFilters}
				onChange={syncFiltersToUrl}
			/>
		{/if}

		<div class="min-h-0 flex-1 overflow-auto">
			{#if loading}
				<div class="p-8 text-center text-[var(--color-muted)]">Loading...</div>
			{:else if error}
				<div class="p-8 text-center text-[var(--color-bad)]">Error: {error}</div>
			{:else if type === 'application' && view === 'kanban'}
				<KanbanBoard
					groupBy={kanbanGroupBy}
					onPick={(id) => goto(`/application/${id}`)}
					onUpdated={(id) => {
						void loadList();
						if (id === selectedFileId) void loadDetail();
					}}
				/>
			{:else if view === 'table'}
				<TableView
					entityType={type}
					{items}
					{details}
					properties={customProperties}
					{visibleCustomKeys}
					{visibleBuiltinKeys}
					selectedFileId={selectedFileId}
					sortKey={propertyFilters.sort}
					sortDir={propertyFilters.sortDir}
					onSort={(key, dir) => {
						syncFiltersToUrl({ ...propertyFilters, sort: key, sortDir: key ? dir : undefined });
					}}
					onUpdated={(id) => {
						void loadList();
						if (id === selectedFileId) void loadDetail();
					}}
				/>
			{:else if type === 'email' && groupBy === 'application'}
				<EmailGroupedList {items} />
			{:else}
				<EntityList {items} {type} {selectedFileId} {details} />
			{/if}
		</div>
	</section>

	<aside class="min-w-0 overflow-hidden bg-[var(--color-panel)]">
		{#if detailLoading}
			<div class="p-8 text-center text-[var(--color-muted)]">Loading detail...</div>
		{:else if selected}
			<DetailPanel
				detail={selected}
				onEditForm={() => (editingForm = true)}
				onEditBody={() => (editingBody = true)}
				onFocus={() => (focusMode = true)}
				onDelete={handleDelete}
			/>
		{:else}
			<div class="flex h-full items-center justify-center p-8 text-center text-sm text-[var(--color-muted)]">
				Select an item to inspect fields, body, backlinks, and actions.
			</div>
		{/if}
	</aside>
</div>

{#if focusMode && selected}
	<div class="fixed inset-0 z-40 flex items-center justify-center bg-black/70 p-8">
		<div class="h-[88vh] w-[min(1120px,92vw)] overflow-hidden rounded-lg border border-[var(--color-border)] bg-[var(--color-panel)] shadow-2xl">
			<DetailPanel
				detail={selected}
				focusMode={true}
				onEditForm={() => (editingForm = true)}
				onEditBody={() => (editingBody = true)}
				onFocus={() => (focusMode = false)}
				onDelete={handleDelete}
			/>
		</div>
	</div>
{/if}

{#if creating}
	<EntityForm
		{type}
		onCancel={() => (creating = false)}
		onDone={(id) => {
			creating = false;
			void loadList();
			goto(`/${type}/${id}`);
		}}
	/>
{/if}

{#if editingForm && selected && fileId}
	<EntityForm
		{type}
		{fileId}
		includeBody={false}
		initial={{ frontmatter: selected.entity, body: selected.body }}
		onCancel={() => (editingForm = false)}
		onDone={() => {
			editingForm = false;
			void loadList();
			void loadDetail();
		}}
	/>
{/if}

{#if editingBody && selected}
	<EntityBodyEditor
		detail={selected}
		onCancel={() => (editingBody = false)}
		onDone={() => {
			editingBody = false;
			void loadList();
			void loadDetail();
		}}
	/>
{/if}

<style>
	.active {
		background-color: rgba(108, 182, 255, 0.15);
		color: var(--color-accent);
	}
</style>
