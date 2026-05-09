<script lang="ts">
	import { goto } from '$app/navigation';
	import { getEntity, listEntities, updateEntity } from '$lib/api/entities';
	import type { ApplicationStatus, EntityListItem } from '$lib/types';
	import { COLOR_CLASSES, type OptionColor } from '$lib/types/schema';

	export interface KanbanColumn {
		value: string;
		label: string;
		color?: OptionColor;
	}

	let {
		groupBy,
		onPick,
		onUpdated
	}: {
		/** When omitted, groups by built-in `status` (the original behavior). */
		groupBy?: { key: string; columns: KanbanColumn[] };
		onPick?: (fileId: string) => void;
		onUpdated?: (fileId: string) => void;
	} = $props();

	const statusColumns: KanbanColumn[] = [
		{ value: 'planning', label: 'planning' },
		{ value: 'drafting', label: 'drafting' },
		{ value: 'submitted', label: 'submitted' },
		{ value: 'decision-pending', label: 'decision-pending' },
		{ value: 'accepted', label: 'accepted', color: 'green' },
		{ value: 'rejected', label: 'rejected', color: 'red' },
		{ value: 'withdrawn', label: 'withdrawn' }
	];

	const groupKey = $derived(groupBy?.key ?? 'status');
	const columnsDef = $derived(groupBy?.columns ?? statusColumns);

	const UNCATEGORIZED: KanbanColumn = { value: '__uncategorized__', label: 'Uncategorized' };

	let columns: Record<string, EntityListItem[]> = $state({});
	let loading = $state(true);

	async function load() {
		loading = true;
		const items = await listEntities('application');
		const next: Record<string, EntityListItem[]> = {};
		for (const c of columnsDef) next[c.value] = [];
		next[UNCATEGORIZED.value] = [];

		await Promise.all(
			items.map(async (item) => {
				try {
					const detail = await getEntity('application', item.file_id);
					const value = (detail.entity as Record<string, unknown>)[groupKey];
					if (typeof value === 'string' && value in next) {
						next[value].push(item);
					} else {
						next[UNCATEGORIZED.value].push(item);
					}
				} catch {
					/* skip */
				}
			})
		);
		columns = next;
		loading = false;
	}

	$effect(() => {
		// re-run when groupBy.key changes
		groupKey;
		void load();
	});

	let dragged: { fileId: string; from: string } | null = null;

	function dragStart(fileId: string, from: string, ev: DragEvent) {
		dragged = { fileId, from };
		ev.dataTransfer?.setData('text/plain', fileId);
	}

	function allowDrop(ev: DragEvent) {
		ev.preventDefault();
	}

	async function drop(ev: DragEvent, target: string) {
		ev.preventDefault();
		if (!dragged || dragged.from === target) {
			dragged = null;
			return;
		}
		const fileId = dragged.fileId;
		dragged = null;
		try {
			const detail = await getEntity('application', fileId);
			const newFm: Record<string, unknown> = { ...(detail.entity as Record<string, unknown>) };
			if (target === UNCATEGORIZED.value) {
				delete newFm[groupKey];
			} else {
				newFm[groupKey] = target;
			}
			await updateEntity('application', fileId, newFm, detail.body);
			await load();
			onUpdated?.(fileId);
		} catch (e) {
			alert(`Failed to update ${groupKey}: ${e instanceof Error ? e.message : String(e)}`);
			await load();
		}
	}

	const allColumns = $derived([...columnsDef, UNCATEGORIZED]);
</script>

{#if loading}
	<div class="p-8 text-center text-[var(--color-muted)]">Loading…</div>
{:else}
	<div class="flex h-full gap-3 overflow-x-auto p-4">
		{#each allColumns as col (col.value)}
			{@const c = col.color ? COLOR_CLASSES[col.color] : null}
			{@const items = columns[col.value] ?? []}
			{#if col.value !== UNCATEGORIZED.value || items.length > 0}
				<div
					class="flex w-64 flex-shrink-0 flex-col rounded border border-[var(--color-border)] bg-[var(--color-panel)]"
					ondragover={allowDrop}
					ondrop={(e) => drop(e, col.value)}
					role="region"
				>
					<header class="flex items-center justify-between border-b border-[var(--color-border)] px-3 py-2">
						<span class="flex items-center gap-2 text-xs font-medium uppercase tracking-wider">
							{#if c}
								<span class="inline-flex h-2 w-2 rounded-full {c.bg} border {c.border}"></span>
							{/if}
							{col.label}
						</span>
						<span class="text-[10px] text-[var(--color-muted)]">{items.length}</span>
					</header>
					<div class="flex flex-1 flex-col gap-2 overflow-auto p-2">
						{#each items as item (item.file_id)}
							<button
								class="rounded border border-[var(--color-border)] bg-white/5 p-2 text-left text-sm hover:border-[var(--color-accent)]"
								draggable={true}
								ondragstart={(e) => dragStart(item.file_id, col.value, e)}
								onclick={() => (onPick ? onPick(item.file_id) : goto(`/application/${item.file_id}`))}
							>
								<div class="truncate font-medium">{item.name}</div>
								<div class="truncate text-xs text-[var(--color-muted)]">{item.file_id}</div>
							</button>
						{/each}
					</div>
				</div>
			{/if}
		{/each}
	</div>
{/if}
