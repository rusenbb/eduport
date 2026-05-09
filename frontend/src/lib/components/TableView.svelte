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
	import { updateEntity } from '$lib/api/entities';
	import { FIELD_DEFS, type FieldDef } from '$lib/entities/meta';
	import type { EntityDetail, EntityListItem, EntityType } from '$lib/types';
	import type { Property, ValueWarning } from '$lib/types/schema';
	import Icon from './Icon.svelte';
	import PropertyCell from './properties/PropertyCell.svelte';
	import PropertyTypeIcon from './properties/PropertyTypeIcon.svelte';

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
		onSort?: (key: string | undefined, dir: 'asc' | 'desc') => void;
		onUpdated?: (fileId: string) => void;
	} = $props();

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
			alert(`Save failed: ${e instanceof Error ? e.message : String(e)}`);
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
		void goto(`/${entityType}/${fileId}`);
	}
</script>

<div class="overflow-x-auto">
	<table class="min-w-full border-collapse text-xs">
		<thead class="sticky top-0 z-10 bg-[var(--color-panel)]">
			<tr class="border-b border-[var(--color-border)]">
				<th
					class="sticky left-0 z-20 cursor-pointer bg-[var(--color-panel)] px-2 py-1.5 text-left font-medium hover:bg-white/5"
					onclick={() => clickHeader('name')}
				>
					<span class="flex items-center gap-1">
						<span>Name</span>
						{#if sortKey === 'name'}
							<span class="text-[10px] text-[var(--color-accent)]">{sortDir === 'desc' ? '↓' : '↑'}</span>
						{/if}
					</span>
				</th>
				{#each visibleBuiltins as field}
					<th
						class="cursor-default border-l border-[var(--color-border)] px-2 py-1.5 text-left font-medium"
					>
						<span class="flex items-center gap-1">
							<Icon name="document" size={11} class="text-[var(--color-muted)]" />
							<span class="truncate">{field.label}</span>
						</span>
					</th>
				{/each}
				{#each visibleCustoms as prop}
					<th
						class="cursor-pointer border-l border-[var(--color-border)] px-2 py-1.5 text-left font-medium hover:bg-white/5"
						onclick={() => clickHeader(prop.key)}
					>
						<span class="flex items-center gap-1">
							<PropertyTypeIcon type={prop.type} class="text-[var(--color-muted)]" />
							<span class="truncate">{prop.name}</span>
							{#if sortKey === prop.key}
								<span class="text-[10px] text-[var(--color-accent)]">{sortDir === 'desc' ? '↓' : '↑'}</span>
							{/if}
						</span>
					</th>
				{/each}
			</tr>
		</thead>
		<tbody>
			{#each items as item (item.file_id)}
				{@const detail = details[item.file_id] ?? null}
				{@const isSelected = item.file_id === selectedFileId}
				<tr
					class="border-b border-[var(--color-border)] hover:bg-white/[0.03]"
					class:bg-blue-500-15={isSelected}
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
								onSave={() => {
									/* read-only */
								}}
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
			{/each}
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
