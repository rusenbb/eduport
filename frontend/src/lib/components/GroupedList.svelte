<script lang="ts">
	/**
	 * Wraps EntityList in collapsible sections grouped by any property
	 * type. The bucket-extraction logic lives in `$lib/utils/viewPipeline`
	 * so List, Table, and Board (Phase C) all behave the same.
	 */
	import type { EntityDetail, EntityListItem, EntityType } from '$lib/types';
	import { COLOR_CLASSES } from '$lib/types/schema';
	import type { Property } from '$lib/types/schema';
	import { groupItems, type GroupGranularity } from '$lib/utils/viewPipeline';
	import { resolveEntity } from '$lib/api/entities';
	import EntityList from './EntityList.svelte';

	let {
		entityType,
		items,
		details = {},
		groupBy,
		dateGranularity = 'month',
		numberStep = 1,
		selectedFileId,
		onContextMenu
	}: {
		entityType: EntityType;
		items: EntityListItem[];
		details?: Record<string, EntityDetail | null>;
		groupBy: Property | null;
		dateGranularity?: GroupGranularity;
		numberStep?: number;
		selectedFileId?: string;
		onContextMenu?: (event: MouseEvent, item: EntityListItem) => void;
	} = $props();

	const buckets = $derived.by(() => {
		if (!groupBy) return null;
		// EntityDetail's `entity` field is unknown-keyed frontmatter;
		// the pipeline reads it as Record<string, unknown>.
		const detailsForPipeline = Object.fromEntries(
			Object.entries(details).map(([k, d]) => [
				k,
				d ? { entity: d.entity as Record<string, unknown> } : null
			])
		);
		return groupItems(items, detailsForPipeline, {
			property: groupBy,
			dateGranularity,
			numberStep
		});
	});

	let collapsed: Record<string, boolean> = $state({});
	function toggle(value: string) {
		collapsed = { ...collapsed, [value]: !collapsed[value] };
	}

	// Bucket-label resolution for relation grouping. The pipeline's
	// bucket value/label for relations is the wikilink target id (e.g.
	// `kyoto-university-…`); resolve to the entity's `name` so the
	// header shows "Kyoto University" instead of the slug.
	let resolvedNames: Record<string, string> = $state({});
	$effect(() => {
		if (!buckets || groupBy?.type !== 'relation') return;
		const targets = buckets
			.map((b) => b.value)
			.filter((v) => v !== '__uncategorized__' && !(v in resolvedNames));
		for (const t of targets) {
			void resolveEntity(t).then(
				(r) => (resolvedNames = { ...resolvedNames, [t]: r.name }),
				() => {}
			);
		}
	});
</script>

{#if !buckets}
	<EntityList {items} type={entityType} {selectedFileId} {details} {onContextMenu} />
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
				<span>{resolvedNames[group.value] ?? group.label}</span>
				<span class="text-[10px] text-[var(--color-muted)]">{group.items.length}</span>
			</button>
			{#if !isCollapsed}
				<EntityList items={group.items} type={entityType} {selectedFileId} {details} {onContextMenu} />
			{/if}
		{/each}
	</div>
{/if}
