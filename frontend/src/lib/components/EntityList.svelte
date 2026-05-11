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
		selection = new Set(),
		onSelectionChange,
		onContextMenu,
		filtersActive = false,
		onClearFilters
	}: {
		items: EntityListItem[];
		type: EntityType;
		selectedFileId?: string;
		details?: Record<string, EntityDetail | null>;
		selection?: Set<string>;
		onSelectionChange?: (next: Set<string>) => void;
		onContextMenu?: (event: MouseEvent, item: EntityListItem) => void;
		filtersActive?: boolean;
		onClearFilters?: () => void;
	} = $props();

	let lastClickedIndex = $state<number | null>(null);

	function navigate(fileId: string) {
		const url = new URL(page.url);
		url.pathname = `/${type}/${fileId}`;
		void goto(url, { keepFocus: true });
	}

	function handleClick(event: MouseEvent, item: EntityListItem, idx: number) {
		const modifierActive = event.ctrlKey || event.metaKey;
		const rangeActive = event.shiftKey;

		// Plain click: clear any multi-selection and navigate.
		if (!modifierActive && !rangeActive) {
			if (selection.size > 0) onSelectionChange?.(new Set());
			lastClickedIndex = idx;
			navigate(item.file_id);
			return;
		}

		const next = new Set(selection);
		if (rangeActive && lastClickedIndex !== null) {
			const [from, to] = [lastClickedIndex, idx].sort((a, b) => a - b);
			for (let i = from; i <= to; i++) next.add(items[i].file_id);
		} else if (modifierActive) {
			if (next.has(item.file_id)) next.delete(item.file_id);
			else next.add(item.file_id);
			lastClickedIndex = idx;
		}
		onSelectionChange?.(next);
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
		{#each items as item, idx (item.file_id)}
			<EntityRow
				{item}
				{type}
				icon={extractIcon(details[item.file_id])}
				selected={item.file_id === selectedFileId}
				multiSelected={selection.has(item.file_id)}
				summary={summarizeItem(item, details[item.file_id])}
				onclick={(event) => handleClick(event, item, idx)}
				{onContextMenu}
			/>
		{/each}
	</div>
{/if}
