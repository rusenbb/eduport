<script lang="ts">
	import type { EntityListItem, EntityType } from '$lib/types';

	let {
		item,
		type,
		icon = '',
		selected = false,
		multiSelected = false,
		summary = '',
		onclick,
		onContextMenu
	}: {
		item: EntityListItem;
		type: EntityType;
		icon?: string;
		selected?: boolean;
		multiSelected?: boolean;
		summary?: string;
		onclick?: (event: MouseEvent) => void;
		onContextMenu?: (event: MouseEvent, item: EntityListItem) => void;
	} = $props();
</script>

<button
	class="grid w-full grid-cols-[auto_1fr_auto] items-center gap-3 border-b border-[var(--color-border)] px-4 py-3 text-left text-sm text-[var(--color-text)] hover:bg-white/[0.025]"
	class:selected
	class:multi-selected={multiSelected}
	onclick={(event) => onclick?.(event)}
	oncontextmenu={(e) => {
		if (onContextMenu) {
			e.preventDefault();
			onContextMenu(e, item);
		}
	}}
>
	{#if icon}
		<span class="text-lg leading-none" aria-hidden="true">{icon}</span>
	{:else}
		<span class="w-0"></span>
	{/if}
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
	.multi-selected {
		background-color: rgba(108, 182, 255, 0.16);
		box-shadow: inset 3px 0 0 var(--color-accent);
	}
</style>
