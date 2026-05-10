<script lang="ts">
	import { status } from '$lib/stores/status';

	let { onSearch, newAction }: { onSearch?: () => void; newAction?: { label: string; onClick: () => void } } = $props();

	// macOS uses ⌘; everyone else uses Ctrl. The layout's keydown handler
	// already accepts either (metaKey || ctrlKey) — this only fixes the
	// label shown in the search button.
	const isMac =
		typeof navigator !== 'undefined' &&
		/Mac|iPhone|iPad|iPod/.test(navigator.platform || navigator.userAgent || '');
	const kbdHint = isMac ? '⌘K' : 'Ctrl+K';
</script>

<div class="flex items-center gap-3 border-b border-[var(--color-border)] px-4 py-2">
	<button
		onclick={onSearch}
		class="flex flex-1 items-center gap-2 rounded border border-[var(--color-border)] bg-[var(--color-panel)] px-3 py-1.5 text-left text-xs text-[var(--color-muted)] hover:border-[var(--color-accent)]"
	>
		<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
			<circle cx="11" cy="11" r="7"></circle>
			<line x1="21" y1="21" x2="16.65" y2="16.65"></line>
		</svg>
		<span class="flex-1">Search across everything</span>
		<kbd class="rounded border border-[var(--color-border)] px-1.5 py-0.5 text-[10px]">{kbdHint}</kbd>
	</button>

	{#if newAction}
		<button
			onclick={newAction.onClick}
			class="rounded border border-blue-700 bg-blue-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-blue-700"
		>
			+ {newAction.label}
		</button>
	{/if}

	{#if !$status.coreUp && $status.lastChecked > 0}
		<span class="rounded bg-red-900/40 px-2 py-1 text-[10px] text-[var(--color-bad)]">Backend offline</span>
	{:else if $status.parseErrors > 0}
		<a href="/status" class="rounded bg-yellow-900/30 px-2 py-1 text-[10px] text-[var(--color-warn)] hover:bg-yellow-900/50">
			{$status.parseErrors} parse errors
		</a>
	{/if}
</div>
