<script lang="ts">
	import { deleteTrashItem, emptyTrash, listTrash, restoreTrashItem } from '$lib/api/trash';
	import { settings } from '$lib/stores/settings';
	import { confirmDestructive } from '$lib/tauri';
	import { toasts } from '$lib/stores/toasts';
	import type { TrashItem } from '$lib/types';

	let items: TrashItem[] = $state([]);
	let loading = $state(true);
	let error: string | null = $state(null);

	async function load() {
		loading = true;
		error = null;
		try {
			items = await listTrash();
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			loading = false;
		}
	}

	async function restore(name: string) {
		try {
			await restoreTrashItem(name);
			await load();
			toasts.success(`Restored "${name}"`);
		} catch (e) {
			toasts.error('Restore failed', e instanceof Error ? e.message : String(e));
		}
	}

	async function remove(name: string) {
		if (!(await confirmDestructive(`Permanently delete ${name}? This cannot be undone.`))) return;
		try {
			await deleteTrashItem(name);
			await load();
			toasts.success(`Deleted "${name}" permanently`);
		} catch (e) {
			toasts.error('Permanent delete failed', e instanceof Error ? e.message : String(e));
		}
	}

	async function clear() {
		if (!(await confirmDestructive(`Permanently empty trash? ${items.length} item${items.length === 1 ? '' : 's'} will be lost forever.`))) return;
		try {
			const n = items.length;
			await emptyTrash();
			await load();
			toasts.success(`Emptied trash`, `${n} item${n === 1 ? '' : 's'} permanently deleted`);
		} catch (e) {
			toasts.error('Empty trash failed', e instanceof Error ? e.message : String(e));
		}
	}

	$effect(() => {
		void load();
	});
</script>

<main class="flex h-full flex-col overflow-hidden p-6">
	<header class="mb-4 flex items-start justify-between gap-4">
		<div>
			<h1 class="text-2xl font-semibold">Trash</h1>
			{#if $settings}
				<p class="mt-1 text-xs text-[var(--color-muted)]">
					{items.length === 0 ? 'Empty' : `${items.length} item${items.length === 1 ? '' : 's'}`}
					· <code>{$settings.data_folder}/.eduport-trash/</code>
				</p>
			{/if}
		</div>
		{#if items.length > 0}
			<button class="rounded border border-red-900 bg-red-900/30 px-3 py-1.5 text-xs text-[var(--color-bad)] hover:bg-red-900/50" onclick={clear}>
				Empty trash
			</button>
		{/if}
	</header>

	{#if loading}
		<div class="p-8 text-center text-[var(--color-muted)]">Loading...</div>
	{:else if error}
		<div class="p-8 text-center text-[var(--color-bad)]">{error}</div>
	{:else if items.length === 0}
		<div class="p-8 text-center text-[var(--color-muted)]">Trash is empty.</div>
	{:else}
		<div class="min-h-0 flex-1 overflow-auto border-y border-[var(--color-border)]">
			{#each items as item}
				<div class="grid grid-cols-[1fr_auto] gap-3 border-b border-[var(--color-border)] px-3 py-3 text-sm last:border-b-0">
					<div class="min-w-0">
						<div class="truncate font-medium">{item.name}</div>
						<div class="truncate text-xs text-[var(--color-muted)]">{item.original_path ?? item.path}</div>
					</div>
					<div class="flex items-center gap-2">
						<button class="rounded border border-[var(--color-border)] px-2 py-1 text-xs hover:bg-white/5" onclick={() => restore(item.name)}>
							Restore
						</button>
						<button class="rounded border border-red-900 px-2 py-1 text-xs text-[var(--color-bad)] hover:bg-red-900/30" onclick={() => remove(item.name)}>
							Delete
						</button>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</main>
