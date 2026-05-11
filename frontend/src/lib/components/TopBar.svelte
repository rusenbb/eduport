<script lang="ts">
	import { status } from '$lib/stores/status';
	import { formatShortcut } from '$lib/keyboard';

	let {
		onSearch,
		onHelp,
		onToggleSidebar,
		sidebarCollapsed = false,
		newAction
	}: {
		onSearch?: () => void;
		onHelp?: () => void;
		onToggleSidebar?: () => void;
		sidebarCollapsed?: boolean;
		newAction?: { label: string; onClick: () => void };
	} = $props();

	const searchHint = formatShortcut(['mod', 'K']);
	const newHint = formatShortcut(['mod', 'N']);
	const sidebarHint = formatShortcut(['mod', '\\']);
</script>

<div class="flex items-center gap-3 border-b border-[var(--color-border)] px-4 py-2">
	{#if onToggleSidebar}
		<button
			onclick={onToggleSidebar}
			class="rounded border border-[var(--color-border)] bg-[var(--color-panel)] px-2 py-1.5 text-xs text-[var(--color-muted)] hover:border-[var(--color-accent)] hover:text-[var(--color-text)]"
			title={`${sidebarCollapsed ? 'Show' : 'Hide'} sidebar (${sidebarHint})`}
			aria-label="Toggle sidebar"
		>
			<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
				<rect x="3" y="3" width="18" height="18" rx="2" />
				<line x1="9" y1="3" x2="9" y2="21" />
			</svg>
		</button>
	{/if}
	<button
		onclick={onSearch}
		class="flex flex-1 items-center gap-2 rounded border border-[var(--color-border)] bg-[var(--color-panel)] px-3 py-1.5 text-left text-xs text-[var(--color-muted)] hover:border-[var(--color-accent)]"
	>
		<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
			<circle cx="11" cy="11" r="7"></circle>
			<line x1="21" y1="21" x2="16.65" y2="16.65"></line>
		</svg>
		<span class="flex-1">Search across everything</span>
		<kbd class="rounded border border-[var(--color-border)] px-1.5 py-0.5 text-[10px]">{searchHint}</kbd>
	</button>

	{#if newAction}
		<button
			onclick={newAction.onClick}
			class="flex items-center gap-1.5 rounded border border-[var(--color-accent)] bg-[var(--color-accent)]/15 px-3 py-1.5 text-xs font-medium text-[var(--color-accent)] hover:bg-[var(--color-accent)]/25"
			title={`Shortcut: ${newHint}`}
		>
			+ {newAction.label}
			<kbd class="rounded border border-[var(--color-accent)]/30 px-1 py-0.5 text-[9px] opacity-70">{newHint}</kbd>
		</button>
	{/if}

	{#if onHelp}
		<button
			onclick={onHelp}
			class="rounded border border-[var(--color-border)] bg-[var(--color-panel)] px-2 py-1.5 text-xs text-[var(--color-muted)] hover:border-[var(--color-accent)] hover:text-[var(--color-text)]"
			title="Keyboard shortcuts (?)"
			aria-label="Keyboard shortcuts"
		>
			?
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
