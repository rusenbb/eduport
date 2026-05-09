<script lang="ts">
	import type { EntityDetail, EntityListItem, EntityType } from '$lib/types';
	import { summarizeItem } from '$lib/entities/meta';
	import EntityRow from './EntityRow.svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';

	let {
		items,
		type,
		selectedFileId,
		details = {}
	}: {
		items: EntityListItem[];
		type: EntityType;
		selectedFileId?: string;
		details?: Record<string, EntityDetail | null>;
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
			<p class="text-[var(--color-muted)]">No {type}s yet.</p>
			<p class="mt-1 text-xs text-[var(--color-muted)]">Use the + New button above to add one.</p>
		</div>
	</div>
{:else}
	<div class="flex flex-col">
		{#each items as item (item.file_id)}
			<EntityRow
				{item}
				{type}
				selected={item.file_id === selectedFileId}
				summary={summarizeItem(item, details[item.file_id])}
				onclick={() => navigate(item.file_id)}
			/>
		{/each}
	</div>
{/if}
