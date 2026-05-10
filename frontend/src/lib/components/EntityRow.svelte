<script lang="ts">
	import type { EntityListItem, EntityType } from '$lib/types';

	let {
		item,
		type,
		selected = false,
		summary = '',
		onclick
	}: {
		item: EntityListItem;
		type: EntityType;
		selected?: boolean;
		summary?: string;
		onclick?: () => void;
	} = $props();
</script>

<button
	class="grid w-full grid-cols-[1fr_auto] items-center gap-3 border-b border-[var(--color-border)] px-4 py-3 text-left text-sm text-[var(--color-text)] hover:bg-white/[0.025]"
	class:selected
	{onclick}
>
	<div class="min-w-0">
		<div class="truncate font-medium">{item.name}</div>
		{#if summary}
			<div class="truncate text-xs text-[var(--color-muted)]">{summary}</div>
		{/if}
	</div>
	<div class="text-[10px] uppercase tracking-wider text-[var(--color-muted)]">{type}</div>
</button>

<style>
	.selected {
		background-color: rgba(108, 182, 255, 0.08);
	}
</style>
