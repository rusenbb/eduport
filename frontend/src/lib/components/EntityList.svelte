<script lang="ts">
	import type { EntityDetail, EntityListItem, EntityType } from '$lib/types';
	import { summarizeItem } from '$lib/entities/meta';
	import { extractIcon } from '$lib/entities/cosmetics';
	import EntityRow from './EntityRow.svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';

	let {
		items,
		type,
		selectedFileId,
		details = {},
		onContextMenu,
		filtersActive = false,
		onClearFilters
	}: {
		items: EntityListItem[];
		type: EntityType;
		selectedFileId?: string;
		details?: Record<string, EntityDetail | null>;
		onContextMenu?: (event: MouseEvent, item: EntityListItem) => void;
		filtersActive?: boolean;
		onClearFilters?: () => void;
	} = $props();

	function navigate(fileId: string) {
		// Preserve query params (view, filters, sort, group, view_id) when
		// jumping into the detail panel so the list view stays as configured.
		const url = new URL(page.url);
		url.pathname = `/${type}/${fileId}`;
		void goto(url, { keepFocus: true });
	}
</script>

{#if items.length === 0}
	<div class="flex h-full items-center justify-center p-8 text-center">
		<div>
			{#if filtersActive}
				<p class="text-[var(--color-muted)]">No {type}s match the current filters.</p>
				{#if onClearFilters}
					<button
						class="mt-2 rounded border border-[var(--color-border)] bg-white/5 px-2 py-1 text-xs hover:bg-white/10"
						onclick={onClearFilters}
					>
						Clear filters
					</button>
				{/if}
			{:else}
				<p class="text-[var(--color-muted)]">No {type}s yet.</p>
				<p class="mt-1 text-xs text-[var(--color-muted)]">Use the + New button above (or press Ctrl/⌘+N).</p>
			{/if}
		</div>
	</div>
{:else}
	<div class="flex flex-col">
		{#each items as item (item.file_id)}
			<EntityRow
				{item}
				{type}
				icon={extractIcon(details[item.file_id])}
				selected={item.file_id === selectedFileId}
				summary={summarizeItem(item, details[item.file_id])}
				onclick={() => navigate(item.file_id)}
				{onContextMenu}
			/>
		{/each}
	</div>
{/if}
