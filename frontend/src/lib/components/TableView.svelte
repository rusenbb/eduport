<script lang="ts">
	/**
	 * Notion-style table view: one row per entity, one column per visible
	 * property (custom + selected built-ins). Cells in *custom* columns
	 * inline-edit on click; built-in cells are read-only here (the detail
	 * panel is the editing surface for built-ins).
	 *
	 * Column visibility is owned by the parent (so it can persist to
	 * localStorage). Sorting is delegated to the parent via the URL —
	 * clicking a header rotates through asc → desc → none.
	 */
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { updateEntity } from '$lib/api/entities';
	import { toasts } from '$lib/stores/toasts';
	import { FIELD_DEFS, builtinKindToPropertyType, type FieldDef } from '$lib/entities/meta';
	import type { EntityDetail, EntityListItem, EntityType } from '$lib/types';
	import type { Property, ValueWarning } from '$lib/types/schema';
	import PropertyCell from './properties/PropertyCell.svelte';
	import PropertyTypeIcon from './properties/PropertyTypeIcon.svelte';

	import { COLOR_CLASSES } from '$lib/types/schema';

	let {
		entityType,
		items,
		details = {},
		properties,
		visibleCustomKeys,
		visibleBuiltinKeys,
		selectedFileId,
		sortKey,
		sortDir,
		groupByKey,
		onSort,
		onUpdated
	}: {
		entityType: EntityType;
		items: EntityListItem[];
		details?: Record<string, EntityDetail | null>;
		properties: Property[];
		visibleCustomKeys: string[];
		visibleBuiltinKeys: string[];
		selectedFileId?: string;
		sortKey?: string;
		sortDir?: 'asc' | 'desc';
		groupByKey?: string;
		onSort?: (key: string | undefined, dir: 'asc' | 'desc') => void;
		onUpdated?: (fileId: string) => void;
	} = $props();

	// Map of groupValue → { label, color, items }. Built only when groupByKey
	// names a single-select property in `properties`. Multi-select / number /
	// date grouping is deferred — Notion supports them but they need bucket
	// logic that isn't worth the complexity for v1.
	const groupBy = $derived.by(() => {
		if (!groupByKey) return null;
		const prop = properties.find((p) => p.key === groupByKey);
		if (!prop || prop.type !== 'single-select') return null;
		return prop;
	});

	const groups = $derived.by(() => {
		if (!groupBy) return null;
		const buckets: Record<
			string,
			{ value: string; label: string; color: string; items: EntityListItem[] }
		> = {};
		for (const opt of groupBy.options) {
			buckets[opt.value] = {
				value: opt.value,
				label: opt.label,
				color: opt.color,
				items: []
			};
		}
		const uncategorized: EntityListItem[] = [];
		for (const item of items) {
			const detail = details[item.file_id];
			const v = (detail?.entity as Record<string, unknown> | undefined)?.[groupBy.key];
			if (typeof v === 'string' && v in buckets) {
				buckets[v].items.push(item);
			} else {
				uncategorized.push(item);
			}
		}
		const ordered = Object.values(buckets);
		if (uncategorized.length > 0) {
			ordered.push({
				value: '__uncategorized__',
				label: 'Uncategorized',
				color: 'gray',
				items: uncategorized
			});
		}
		return ordered;
	});

	let collapsed: Record<string, boolean> = $state({});
	function toggleCollapse(value: string) {
		collapsed = { ...collapsed, [value]: !collapsed[value] };
	}

	const builtinFields = $derived(FIELD_DEFS[entityType] ?? []);

	const visibleBuiltins = $derived(
		visibleBuiltinKeys
			.map((k) => builtinFields.find((f) => f.key === k))
			.filter((f): f is FieldDef => !!f)
	);

	const visibleCustoms = $derived(
		visibleCustomKeys
			.map((k) => properties.find((p) => p.key === k))
			.filter((p): p is Property => !!p)
	);

	let editingCell: { fileId: string; key: string } | null = $state(null);

	function startEdit(fileId: string, key: string) {
		editingCell = { fileId, key };
	}

	function cancelEdit() {
		editingCell = null;
	}

	async function saveEdit(fileId: string, key: string, next: unknown) {
		const detail = details[fileId];
		if (!detail) {
			editingCell = null;
			return;
		}
		const newFm: Record<string, unknown> = { ...(detail.entity as Record<string, unknown>) };
		if (
			next === null ||
			next === undefined ||
			next === '' ||
			(Array.isArray(next) && next.length === 0)
		) {
			delete newFm[key];
		} else {
			newFm[key] = next;
		}
		try {
			await updateEntity(entityType, fileId, newFm, detail.body);
			onUpdated?.(fileId);
		} catch (e) {
			toasts.error('Save failed', e instanceof Error ? e.message : String(e));
		} finally {
			editingCell = null;
		}
	}

	function warningsFor(detail: EntityDetail | null, key: string): ValueWarning[] {
		return (detail?.value_warnings ?? []).filter((w) => w.key === key);
	}

	function clickHeader(key: string) {
		if (!onSort) return;
		if (sortKey !== key) {
			onSort(key, 'asc');
		} else if (sortDir === 'asc') {
			onSort(key, 'desc');
		} else {
			onSort(undefined, 'asc');
		}
	}

	function navigate(fileId: string) {
		// Preserve all query params (view=table, filters, sort, group, view_id)
		// so opening an entity in the detail panel keeps the surrounding view
		// configured as the user left it.
		const url = new URL(page.url);
		url.pathname = `/${entityType}/${fileId}`;
		void goto(url, { keepFocus: true });
	}
</script>

<div class="overflow-x-auto">
	<table class="min-w-full border-collapse text-xs">
		<thead class="sticky top-0 z-10 bg-[var(--color-panel)]">
			<tr class="border-b border-[var(--color-border)]">
				<th
					class="sticky left-0 z-20 group cursor-pointer bg-[var(--color-panel)] px-2 py-1.5 text-left font-medium hover:bg-white/5"
					onclick={() => clickHeader('name')}
					title="Click to sort"
				>
					<span class="flex items-center gap-1">
						<span>Name</span>
						{#if sortKey === 'name'}
							<span class="text-[10px] text-[var(--color-accent)]">{sortDir === 'desc' ? '↓' : '↑'}</span>
						{:else}
							<span class="text-[10px] text-[var(--color-muted)] opacity-0 transition-opacity group-hover:opacity-100">↕</span>
						{/if}
					</span>
				</th>
				{#each visibleBuiltins as field}
					<th
						class="cursor-default border-l border-[var(--color-border)] px-2 py-1.5 text-left font-medium"
						title="Built-in field — edit on the entity's detail panel"
					>
						<span class="flex items-center gap-1">
							<PropertyTypeIcon
								type={builtinKindToPropertyType(field.kind)}
								class="text-[var(--color-muted)]"
							/>
							<span class="truncate">{field.label}</span>
						</span>
					</th>
				{/each}
				{#each visibleCustoms as prop}
					<th
						class="group cursor-pointer border-l border-[var(--color-border)] px-2 py-1.5 text-left font-medium hover:bg-white/5"
						onclick={() => clickHeader(prop.key)}
						title="Click to sort"
					>
						<span class="flex items-center gap-1">
							<PropertyTypeIcon type={prop.type} class="text-[var(--color-muted)]" />
							<span class="truncate">{prop.name}</span>
							{#if sortKey === prop.key}
								<span class="text-[10px] text-[var(--color-accent)]">{sortDir === 'desc' ? '↓' : '↑'}</span>
							{:else}
								<span class="text-[10px] text-[var(--color-muted)] opacity-0 transition-opacity group-hover:opacity-100">↕</span>
							{/if}
						</span>
					</th>
				{/each}
			</tr>
		</thead>
		<tbody>
			{#snippet entityRow(item: EntityListItem)}
				{@const detail = details[item.file_id] ?? null}
				{@const isSelected = item.file_id === selectedFileId}
				<tr
					class="border-b border-[var(--color-border)] hover:bg-white/[0.03]"
					class:selected={isSelected}
				>
					<td
						class="sticky left-0 z-10 cursor-pointer bg-[var(--color-panel)] px-2 py-1 hover:bg-white/5"
						class:selected-cell={isSelected}
						onclick={() => navigate(item.file_id)}
					>
						<div class="truncate font-medium">{item.name}</div>
						<div class="truncate text-[10px] text-[var(--color-muted)]">{item.file_id}</div>
					</td>
					{#each visibleBuiltins as field}
						{@const value = (detail?.entity as Record<string, unknown> | undefined)?.[field.key]}
						<td
							class="cursor-pointer border-l border-[var(--color-border)] align-top"
							onclick={() => navigate(item.file_id)}
						>
							<PropertyCell
								column={{ kind: 'builtin', field }}
								{value}
								editing={false}
								onStartEdit={() => navigate(item.file_id)}
								onSave={() => {}}
								onCancel={cancelEdit}
							/>
						</td>
					{/each}
					{#each visibleCustoms as prop}
						{@const value = (detail?.entity as Record<string, unknown> | undefined)?.[prop.key]}
						{@const editing = editingCell?.fileId === item.file_id && editingCell?.key === prop.key}
						<td class="border-l border-[var(--color-border)] align-top">
							<PropertyCell
								column={{ kind: 'custom', prop }}
								{value}
								warnings={warningsFor(detail, prop.key)}
								{editing}
								onStartEdit={() => startEdit(item.file_id, prop.key)}
								onSave={(next) => saveEdit(item.file_id, prop.key, next)}
								onCancel={cancelEdit}
							/>
						</td>
					{/each}
				</tr>
			{/snippet}

			{#if groups}
				{@const colSpan = 1 + visibleBuiltins.length + visibleCustoms.length}
				{#each groups as group (group.value)}
					{@const c = COLOR_CLASSES[group.color as keyof typeof COLOR_CLASSES] ?? COLOR_CLASSES.gray}
					{@const isCollapsed = collapsed[group.value]}
					<tr class="border-y border-[var(--color-border)] bg-[var(--color-bg)]">
						<td
							class="sticky left-0 z-10 cursor-pointer bg-[var(--color-bg)] px-2 py-1.5 text-xs font-medium hover:bg-white/5"
							{...{ colspan: colSpan } as unknown as Record<string, never>}
							onclick={() => toggleCollapse(group.value)}
						>
							<span class="inline-flex items-center gap-2">
								<span class="text-[10px] text-[var(--color-muted)]">{isCollapsed ? '▶' : '▼'}</span>
								<span class="inline-flex h-2 w-2 rounded-full {c.bg} border {c.border}"></span>
								<span>{group.label}</span>
								<span class="text-[10px] text-[var(--color-muted)]">{group.items.length}</span>
							</span>
						</td>
					</tr>
					{#if !isCollapsed}
						{#each group.items as item (item.file_id)}
							{@render entityRow(item)}
						{/each}
					{/if}
				{/each}
			{:else}
				{#each items as item (item.file_id)}
					{@render entityRow(item)}
				{/each}
			{/if}
		</tbody>
	</table>

	{#if items.length === 0}
		<div class="p-8 text-center text-[var(--color-muted)]">No items.</div>
	{/if}
</div>

<style>
	.selected {
		background-color: rgba(108, 182, 255, 0.08);
	}
	.selected-cell {
		background-color: rgba(108, 182, 255, 0.12);
	}
</style>
