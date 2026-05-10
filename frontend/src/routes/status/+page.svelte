<script lang="ts">
	import { listParseErrors } from '$lib/api/status';
	import type { ParseErrorItem } from '$lib/types';

	let errors: ParseErrorItem[] = $state([]);
	let loading = $state(true);
	let message: string | null = $state(null);

	async function load() {
		loading = true;
		message = null;
		try {
			errors = await listParseErrors();
		} catch (e) {
			message = e instanceof Error ? e.message : String(e);
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		void load();
	});
</script>

<main class="p-6">
	<header class="mb-5">
		<h1 class="text-2xl font-semibold">Status</h1>
		<p class="mt-1 text-sm text-[var(--color-muted)]">Parser errors and backend health surfaced from the vault index.</p>
	</header>

	{#if loading}
		<div class="p-8 text-center text-[var(--color-muted)]">Loading...</div>
	{:else if message}
		<div class="p-8 text-center text-[var(--color-bad)]">{message}</div>
	{:else if errors.length === 0}
		<div class="border-y border-[var(--color-border)] py-8 text-center text-sm text-[var(--color-good)]">
			No parse errors.
		</div>
	{:else}
		<div class="border-y border-[var(--color-border)]">
			{#each errors as error}
				<div class="border-b border-[var(--color-border)] px-3 py-3 last:border-b-0">
					<div class="font-mono text-xs text-[var(--color-warn)]">{error.path}</div>
					<div class="mt-1 text-sm">{error.message}</div>
					<div class="mt-1 text-[10px] uppercase tracking-wider text-[var(--color-muted)]">{error.occurred_at}</div>
				</div>
			{/each}
		</div>
	{/if}
</main>
