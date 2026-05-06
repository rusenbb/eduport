<script lang="ts">
	import { status } from '$lib/stores/status';

	let { onSearch, newAction }: { onSearch?: () => void; newAction?: { label: string; onClick: () => void } } = $props();
</script>

<div class="flex items-center gap-3 border-b border-[var(--color-border)] px-4 py-2">
	<button
		onclick={onSearch}
		class="flex flex-1 items-center gap-2 rounded border border-[var(--color-border)] bg-[var(--color-panel)] px-3 py-1.5 text-left text-xs text-[var(--color-muted)] hover:border-[var(--color-accent)]"
	>
		<span>🔍</span>
		<span class="flex-1">Search across everything</span>
		<kbd class="rounded border border-[var(--color-border)] px-1.5 py-0.5 text-[10px]">⌘K</kbd>
	</button>

	{#if newAction}
		<button
			onclick={newAction.onClick}
			class="rounded border border-blue-700 bg-blue-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-blue-700"
		>
			+ {newAction.label}
		</button>
	{/if}

	{#if !$status.sidecarUp && $status.lastChecked > 0}
		<span class="rounded bg-red-900/40 px-2 py-1 text-[10px] text-[var(--color-bad)]">Backend offline</span>
	{/if}
</div>
