<script lang="ts">
	import Icon from './Icon.svelte';
	import { formatShortcut } from '$lib/keyboard';

	let { open = $bindable(false) }: { open: boolean } = $props();

	interface ShortcutRow {
		keys: string[];
		label: string;
	}

	const GROUPS: { title: string; rows: ShortcutRow[] }[] = [
		{
			title: 'Global',
			rows: [
				{ keys: ['mod', 'K'], label: 'Search across everything' },
				{ keys: ['mod', 'N'], label: 'New entity (in the current type)' },
				{ keys: ['?'], label: 'Show this help' },
				{ keys: ['esc'], label: 'Close any open dialog' }
			]
		},
		{
			title: 'List navigation',
			rows: [
				{ keys: ['J'], label: 'Move selection down (or ↓)' },
				{ keys: ['K'], label: 'Move selection up (or ↑)' },
				{ keys: ['enter'], label: 'Open the focused item' },
				{ keys: ['G'], label: 'Jump to first item' },
				{ keys: ['shift', 'G'], label: 'Jump to last item' }
			]
		},
		{
			title: 'Detail panel',
			rows: [
				{ keys: ['E'], label: 'Edit fields' },
				{ keys: ['shift', 'E'], label: 'Edit body' },
				{ keys: ['F'], label: 'Toggle focus mode' },
				{ keys: ['mod', 'backspace'], label: 'Move entity to trash' }
			]
		}
	];

	function onKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			event.preventDefault();
			open = false;
		}
	}
</script>

<svelte:window onkeydown={onKeydown} />

{#if open}
	<!-- Backdrop is a regular div with a click handler that only fires
		 on direct clicks (not bubble-through from the inner dialog).
		 Dialog role is on the inner panel; Esc handler is on window. -->
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-6"
		role="presentation"
		onclick={(e) => {
			if (e.target === e.currentTarget) open = false;
		}}
	>
		<div
			class="flex w-[min(520px,92vw)] flex-col overflow-hidden rounded-lg border border-[var(--color-border)] bg-[var(--color-panel)] shadow-2xl"
			role="dialog"
			aria-modal="true"
			aria-label="Keyboard shortcuts"
			tabindex="-1"
		>
			<header class="flex items-center justify-between border-b border-[var(--color-border)] px-4 py-3">
				<h2 class="text-sm font-semibold">Keyboard shortcuts</h2>
				<button
					class="rounded p-1 hover:bg-white/5"
					onclick={() => (open = false)}
					aria-label="Close"
				>
					<Icon name="x" />
				</button>
			</header>
			<div class="grid gap-5 p-4 text-xs">
				{#each GROUPS as group}
					<section>
						<h3 class="mb-2 text-[10px] uppercase tracking-wider text-[var(--color-muted)]">
							{group.title}
						</h3>
						<div class="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1.5">
							{#each group.rows as row}
								<kbd class="self-start rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-1.5 py-0.5 font-mono">
									{formatShortcut(row.keys)}
								</kbd>
								<span class="self-center">{row.label}</span>
							{/each}
						</div>
					</section>
				{/each}
			</div>
			<footer class="border-t border-[var(--color-border)] bg-[var(--color-bg)]/40 px-4 py-2 text-[10px] text-[var(--color-muted)]">
				List shortcuts (J/K/Enter/G) are disabled while typing in a field.
			</footer>
		</div>
	</div>
{/if}
