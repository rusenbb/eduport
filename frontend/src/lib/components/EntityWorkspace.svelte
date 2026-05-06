<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { getContext } from 'svelte';
	import type { Writable } from 'svelte/store';
	import { deleteEntity, getEntity, listEntities } from '$lib/api/entities';
	import { filters } from '$lib/stores/filters';
	import { settings } from '$lib/stores/settings';
	import type { EntityDetail, EntityListItem, EntityType } from '$lib/types';
	import { TYPE_LABELS } from '$lib/entities/meta';
	import { confirmDestructive } from '$lib/tauri';
	import EntityBodyEditor from './EntityBodyEditor.svelte';
	import DetailPanel from './DetailPanel.svelte';
	import EntityForm from './EntityForm.svelte';
	import EntityList from './EntityList.svelte';
	import KanbanBoard from './KanbanBoard.svelte';

	let { type, fileId = null }: { type: EntityType; fileId?: string | null } = $props();

	let items: EntityListItem[] = $state([]);
	let details: Record<string, EntityDetail | null> = $state({});
	let selected: EntityDetail | null = $state(null);
	let loading = $state(true);
	let detailLoading = $state(false);
	let error: string | null = $state(null);
	let creating = $state(false);
	let editingForm = $state(false);
	let editingBody = $state(false);
	let focusMode = $state(false);
	const view = $derived(page.url.searchParams.get('view') === 'kanban' ? 'kanban' : 'list');

	function setView(next: 'list' | 'kanban') {
		const url = new URL(page.url);
		if (next === 'list') url.searchParams.delete('view');
		else url.searchParams.set('view', next);
		void goto(url, { replaceState: true, keepFocus: true, noScroll: true });
	}

	const newAction = getContext<Writable<{ label: string; onClick: () => void } | null>>('eduport:newAction');
	const selectedFileId = $derived(fileId ?? undefined);

	async function loadList() {
		loading = true;
		error = null;
		try {
			items = await listEntities(type, $filters.tags);
			const nextDetails: Record<string, EntityDetail | null> = {};
			await Promise.all(
				items.map(async (item) => {
					try {
						nextDetails[item.file_id] = await getEntity(type, item.file_id);
					} catch {
						nextDetails[item.file_id] = null;
					}
				})
			);
			details = nextDetails;
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			loading = false;
		}
	}

	async function loadDetail() {
		if (!fileId) {
			selected = null;
			return;
		}
		detailLoading = true;
		try {
			selected = await getEntity(type, fileId);
			details = { ...details, [fileId]: selected };
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
			selected = null;
		} finally {
			detailLoading = false;
		}
	}

	$effect(() => {
		const _filterKey = $filters.tags.join('\u0000');
		newAction?.set({ label: `New ${TYPE_LABELS[type]}`, onClick: () => (creating = true) });
		void loadList();
		return () => newAction?.set(null);
	});

	$effect(() => {
		void loadDetail();
	});

	async function handleDelete() {
		if (!selected || !fileId) return;
		if (($settings?.confirm_deletes ?? true) && !(await confirmDestructive(`Move "${selected.entity.name}" to trash?`))) return;
		try {
			await deleteEntity(type, fileId);
			selected = null;
			focusMode = false;
			await loadList();
			goto(`/${type}`);
		} catch (e) {
			alert(`Delete failed: ${e instanceof Error ? e.message : String(e)}`);
		}
	}
</script>

<div class="grid h-full min-h-0 grid-cols-[minmax(360px,1fr)_minmax(360px,440px)] overflow-hidden">
	<section class="flex min-w-0 flex-col overflow-hidden border-r border-[var(--color-border)]">
		<header class="flex items-center justify-between border-b border-[var(--color-border)] px-4 py-2">
			<div>
				<h1 class="text-sm font-semibold">{TYPE_LABELS[type]}</h1>
				<p class="text-[10px] uppercase tracking-wider text-[var(--color-muted)]">{items.length} items</p>
			</div>
			{#if type === 'application'}
				<div class="flex rounded border border-[var(--color-border)] p-0.5 text-xs">
					<button class="rounded px-2 py-1" class:active={view === 'list'} onclick={() => setView('list')}>List</button>
					<button class="rounded px-2 py-1" class:active={view === 'kanban'} onclick={() => setView('kanban')}>Kanban</button>
				</div>
			{/if}
		</header>

		<div class="min-h-0 flex-1 overflow-auto">
			{#if loading}
				<div class="p-8 text-center text-[var(--color-muted)]">Loading...</div>
			{:else if error}
				<div class="p-8 text-center text-[var(--color-bad)]">Error: {error}</div>
			{:else if type === 'application' && view === 'kanban'}
				<KanbanBoard
					onPick={(id) => goto(`/application/${id}`)}
					onUpdated={(id) => {
						void loadList();
						if (id === selectedFileId) void loadDetail();
					}}
				/>
			{:else}
				<EntityList {items} {type} {selectedFileId} {details} />
			{/if}
		</div>
	</section>

	<aside class="min-w-0 overflow-hidden bg-[var(--color-panel)]">
		{#if detailLoading}
			<div class="p-8 text-center text-[var(--color-muted)]">Loading detail...</div>
		{:else if selected}
			<DetailPanel
				detail={selected}
				onEditForm={() => (editingForm = true)}
				onEditBody={() => (editingBody = true)}
				onFocus={() => (focusMode = true)}
				onDelete={handleDelete}
			/>
		{:else}
			<div class="flex h-full items-center justify-center p-8 text-center text-sm text-[var(--color-muted)]">
				Select an item to inspect fields, body, backlinks, and actions.
			</div>
		{/if}
	</aside>
</div>

{#if focusMode && selected}
	<div class="fixed inset-0 z-40 flex items-center justify-center bg-black/70 p-8">
		<div class="h-[88vh] w-[min(1120px,92vw)] overflow-hidden rounded-lg border border-[var(--color-border)] bg-[var(--color-panel)] shadow-2xl">
			<DetailPanel
				detail={selected}
				focusMode={true}
				onEditForm={() => (editingForm = true)}
				onEditBody={() => (editingBody = true)}
				onFocus={() => (focusMode = false)}
				onDelete={handleDelete}
			/>
		</div>
	</div>
{/if}

{#if creating}
	<EntityForm
		{type}
		onCancel={() => (creating = false)}
		onDone={(id) => {
			creating = false;
			void loadList();
			goto(`/${type}/${id}`);
		}}
	/>
{/if}

{#if editingForm && selected && fileId}
	<EntityForm
		{type}
		{fileId}
		includeBody={false}
		initial={{ frontmatter: selected.entity, body: selected.body }}
		onCancel={() => (editingForm = false)}
		onDone={() => {
			editingForm = false;
			void loadList();
			void loadDetail();
		}}
	/>
{/if}

{#if editingBody && selected}
	<EntityBodyEditor
		detail={selected}
		onCancel={() => (editingBody = false)}
		onDone={() => {
			editingBody = false;
			void loadList();
			void loadDetail();
		}}
	/>
{/if}

<style>
	.active {
		background-color: rgba(108, 182, 255, 0.15);
		color: var(--color-accent);
	}
</style>
