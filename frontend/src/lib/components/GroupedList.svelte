<script lang="ts">
	/**
	 * Wraps EntityList in collapsible sections grouped by a single-select
	 * custom property. Falls back to a flat list when groupBy is null.
	 */
	import type { EntityDetail, EntityListItem, EntityType } from '$lib/types';
	import { COLOR_CLASSES, type SingleSelectProperty } from '$lib/types/schema';
	import EntityList from './EntityList.svelte';

	let {
		entityType,
		items,
		details = {},
		groupBy,
		selectedFileId
	}: {
		entityType: EntityType;
		items: EntityListItem[];
		details?: Record<string, EntityDetail | null>;
		groupBy: SingleSelectProperty | null;
		selectedFileId?: string;
	} = $props();

	const buckets = $derived.by(() => {
		if (!groupBy) return null;
		const map: Record<
			string,
			{ value: string; label: string; color: string; items: EntityListItem[] }
		> = {};
		for (const opt of groupBy.options) {
			map[opt.value] = { value: opt.value, label: opt.label, color: opt.color, items: [] };
		}
		const uncategorized: EntityListItem[] = [];
		for (const item of items) {
			const detail = details[item.file_id];
			const v = (detail?.entity as Record<string, unknown> | undefined)?.[groupBy.key];
			if (typeof v === 'string' && v in map) {
				map[v].items.push(item);
			} else {
				uncategorized.push(item);
			}
		}
		const list = Object.values(map);
		if (uncategorized.length > 0) {
			list.push({
				value: '__uncategorized__',
				label: 'Uncategorized',
				color: 'gray',
				items: uncategorized
			});
		}
		return list;
	});

	let collapsed: Record<string, boolean> = $state({});
	function toggle(value: string) {
		collapsed = { ...collapsed, [value]: !collapsed[value] };
	}
</script>

{#if !buckets}
	<EntityList {items} type={entityType} {selectedFileId} {details} />
{:else}
	<div>
		{#each buckets as group (group.value)}
			{@const c = COLOR_CLASSES[group.color as keyof typeof COLOR_CLASSES] ?? COLOR_CLASSES.gray}
			{@const isCollapsed = collapsed[group.value]}
			<button
				class="flex w-full items-center gap-2 border-b border-[var(--color-border)] bg-[var(--color-bg)] px-4 py-1.5 text-left text-xs font-medium hover:bg-white/5"
				onclick={() => toggle(group.value)}
			>
				<span class="text-[10px] text-[var(--color-muted)]">{isCollapsed ? '▶' : '▼'}</span>
				<span class="inline-flex h-2 w-2 rounded-full {c.bg} border {c.border}"></span>
				<span>{group.label}</span>
				<span class="text-[10px] text-[var(--color-muted)]">{group.items.length}</span>
			</button>
			{#if !isCollapsed}
				<EntityList items={group.items} type={entityType} {selectedFileId} {details} />
			{/if}
		{/each}
	</div>
{/if}
