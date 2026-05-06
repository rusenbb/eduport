<script lang="ts">
	import { goto } from '$app/navigation';
	import { search } from '$lib/api/search';
	import type { SearchHit } from '$lib/types';

	let { open = $bindable(false) }: { open?: boolean } = $props();

	let query = $state('');
	let hits: SearchHit[] = $state([]);
	let activeIndex = $state(0);
	let loading = $state(false);

	let debounce: ReturnType<typeof setTimeout> | null = null;

	$effect(() => {
		if (!open) {
			query = '';
			hits = [];
			activeIndex = 0;
			return;
		}
	});

	$effect(() => {
		if (debounce !== null) clearTimeout(debounce);
		const q = query.trim();
		if (q.length === 0) {
			hits = [];
			return;
		}
		loading = true;
		debounce = setTimeout(async () => {
			try {
				hits = await search(q, 20);
				activeIndex = 0;
			} catch {
				hits = [];
			} finally {
				loading = false;
			}
		}, 150);
	});

	function close() {
		open = false;
	}

	function pick(hit: SearchHit) {
		close();
		goto(`/${hit.type}/${encodeURIComponent(hit.file_id)}`);
	}

	function onKey(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			close();
		} else if (event.key === 'ArrowDown') {
			event.preventDefault();
			activeIndex = Math.min(activeIndex + 1, hits.length - 1);
		} else if (event.key === 'ArrowUp') {
			event.preventDefault();
			activeIndex = Math.max(activeIndex - 1, 0);
		} else if (event.key === 'Enter' && hits[activeIndex]) {
			event.preventDefault();
			pick(hits[activeIndex]);
		}
	}
</script>

{#if open}
	<div class="fixed inset-0 z-50 flex items-start justify-center bg-black/60 pt-[15vh]" onclick={close} role="presentation">
		<div
			class="flex w-[640px] max-w-[90vw] flex-col overflow-hidden rounded-lg border border-[var(--color-border)] bg-[var(--color-panel)] shadow-2xl"
			onclick={(e) => e.stopPropagation()}
			role="dialog"
		>
			<input
				bind:value={query}
				onkeydown={onKey}
				placeholder="Search across all entities…"
				class="border-b border-[var(--color-border)] bg-transparent px-4 py-3 text-sm outline-none"
				autofocus
			/>
			<div class="max-h-[50vh] overflow-auto">
				{#if loading}
					<div class="p-4 text-center text-xs text-[var(--color-muted)]">Searching…</div>
				{:else if hits.length === 0 && query.trim().length > 0}
					<div class="p-4 text-center text-xs text-[var(--color-muted)]">No results.</div>
				{:else}
					{#each hits as hit, idx}
						<button
							class="block w-full border-b border-[var(--color-border)] px-4 py-2 text-left text-sm hover:bg-white/5"
							class:active={idx === activeIndex}
							onclick={() => pick(hit)}
						>
							<div class="font-medium">{hit.name}</div>
							<div class="text-[10px] uppercase text-[var(--color-muted)]">{hit.type} · {hit.file_id}</div>
							{#if hit.snippet}
								<div class="mt-1 text-xs text-[var(--color-muted)]">{@html hit.snippet}</div>
							{/if}
						</button>
					{/each}
				{/if}
			</div>
		</div>
	</div>
{/if}

<style>
	.active {
		background-color: rgba(108, 182, 255, 0.1);
	}
</style>
