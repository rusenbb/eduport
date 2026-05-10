<script lang="ts">
	import { listParseErrors } from '$lib/api/status';
	import { listenCoreEvent } from '$lib/api/client';
	import { isTauri, revealInFileManager } from '$lib/tauri';
	import { toasts } from '$lib/stores/toasts';
	import { onMount } from 'svelte';
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

	// Auto-refresh whenever the index changes — when the user edits a
	// file with a parse error and saves, the error should disappear
	// here without a manual reload. parse_errors are not in the
	// watcher's typed events but every entity_changed / needs_rescan /
	// schema_changed triggers a fresh parse pass.
	onMount(() => {
		if (!isTauri()) return;
		let unlisten: (() => void) | null = null;
		let pending: ReturnType<typeof setTimeout> | null = null;
		void listenCoreEvent<{ kind: string }>('eduport:vault-event', () => {
			if (pending) clearTimeout(pending);
			pending = setTimeout(() => {
				pending = null;
				void load();
			}, 250);
		}).then((u) => {
			unlisten = u;
		});
		return () => {
			if (pending) clearTimeout(pending);
			unlisten?.();
		};
	});

	async function reveal(path: string) {
		try {
			await revealInFileManager(path);
		} catch (e) {
			toasts.error('Reveal failed', e instanceof Error ? e.message : String(e));
		}
	}
</script>

<main class="p-6">
	<header class="mb-5 flex items-start justify-between gap-4">
		<div>
			<h1 class="text-2xl font-semibold">Status</h1>
			<p class="mt-1 text-sm text-[var(--color-muted)]">Parser errors and backend health surfaced from the vault index.</p>
		</div>
		{#if !loading && errors.length > 0}
			<span class="rounded border border-[var(--color-warn)]/40 bg-[var(--color-warn)]/10 px-2 py-1 text-xs text-[var(--color-warn)]">
				{errors.length} parse error{errors.length === 1 ? '' : 's'}
			</span>
		{/if}
	</header>

	{#if loading}
		<div class="p-8 text-center text-[var(--color-muted)]">Loading...</div>
	{:else if message}
		<div class="p-8 text-center text-[var(--color-bad)]">{message}</div>
	{:else if errors.length === 0}
		<div class="border-y border-[var(--color-border)] py-8 text-center text-sm text-[var(--color-good)]">
			No parse errors. The vault is clean.
		</div>
	{:else}
		<div class="border-y border-[var(--color-border)]">
			{#each errors as error}
				<div class="grid grid-cols-[1fr_auto] items-start gap-3 border-b border-[var(--color-border)] px-3 py-3 last:border-b-0">
					<div class="min-w-0">
						<div class="truncate font-mono text-xs text-[var(--color-warn)]">{error.path}</div>
						<div class="mt-1 text-sm">{error.message}</div>
						<div class="mt-1 text-[10px] uppercase tracking-wider text-[var(--color-muted)]">{error.occurred_at}</div>
					</div>
					<button
						class="flex-shrink-0 rounded border border-[var(--color-border)] bg-white/5 px-2 py-1 text-xs hover:bg-white/10"
						onclick={() => reveal(error.path)}
						title="Open the file's folder and select it"
					>
						Show in file manager
					</button>
				</div>
			{/each}
		</div>
	{/if}
</main>
