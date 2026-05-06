<script lang="ts">
	import type { EntityDetail, EntityListItem, EntityType } from '$lib/types';
	import { summarizeItem } from '$lib/entities/meta';
	import EntityRow from './EntityRow.svelte';
	import { goto } from '$app/navigation';

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
				onclick={() => goto(`/${type}/${item.file_id}`)}
			/>
		{/each}
	</div>
{/if}
