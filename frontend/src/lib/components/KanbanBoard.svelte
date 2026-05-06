<script lang="ts">
	import { goto } from '$app/navigation';
	import { getEntity, listEntities, updateEntity } from '$lib/api/entities';
	import type { ApplicationStatus, EntityListItem } from '$lib/types';

	let { onPick }: { onPick?: (fileId: string) => void } = $props();

	const statuses: ApplicationStatus[] = [
		'planning',
		'drafting',
		'submitted',
		'decision-pending',
		'accepted',
		'rejected',
		'withdrawn'
	];

	let columns: Record<ApplicationStatus, { item: EntityListItem; status: ApplicationStatus }[]> = $state({
		planning: [],
		drafting: [],
		submitted: [],
		'decision-pending': [],
		accepted: [],
		rejected: [],
		withdrawn: []
	});
	let loading = $state(true);

	async function load() {
		loading = true;
		const items = await listEntities('application');
		const byStatus = Object.fromEntries(statuses.map((s) => [s, [] as { item: EntityListItem; status: ApplicationStatus }[]])) as typeof columns;
		// Fetch each entity's status; this is N+1 but fine for v1 scale.
		await Promise.all(
			items.map(async (item) => {
				try {
					const detail = await getEntity('application', item.file_id);
					const status = (detail.entity.status as ApplicationStatus) ?? 'planning';
					byStatus[status]?.push({ item, status });
				} catch {
					// skip
				}
			})
		);
		columns = byStatus;
		loading = false;
	}

	$effect(() => {
		void load();
	});

	let dragged: { fileId: string; from: ApplicationStatus } | null = null;

	function dragStart(fileId: string, from: ApplicationStatus, ev: DragEvent) {
		dragged = { fileId, from };
		ev.dataTransfer?.setData('text/plain', fileId);
	}

	function allowDrop(ev: DragEvent) {
		ev.preventDefault();
	}

	async function drop(ev: DragEvent, target: ApplicationStatus) {
		ev.preventDefault();
		if (!dragged || dragged.from === target) {
			dragged = null;
			return;
		}
		const fileId = dragged.fileId;
		dragged = null;
		try {
			const detail = await getEntity('application', fileId);
			const newFm = { ...(detail.entity as Record<string, unknown>), status: target };
			await updateEntity('application', fileId, newFm, detail.body);
			await load();
		} catch (e) {
			alert(`Failed to update status: ${e instanceof Error ? e.message : String(e)}`);
			await load();
		}
	}
</script>

{#if loading}
	<div class="p-8 text-center text-[var(--color-muted)]">Loading…</div>
{:else}
	<div class="flex h-full gap-3 overflow-x-auto p-4">
		{#each statuses as status}
			<div
				class="flex w-64 flex-shrink-0 flex-col rounded border border-[var(--color-border)] bg-[var(--color-panel)]"
				ondragover={allowDrop}
				ondrop={(e) => drop(e, status)}
				role="region"
			>
				<header class="flex items-center justify-between border-b border-[var(--color-border)] px-3 py-2">
					<span class="text-xs font-medium uppercase tracking-wider">{status}</span>
					<span class="text-[10px] text-[var(--color-muted)]">{columns[status].length}</span>
				</header>
				<div class="flex flex-1 flex-col gap-2 overflow-auto p-2">
					{#each columns[status] as { item }}
						<button
							class="rounded border border-[var(--color-border)] bg-white/5 p-2 text-left text-sm hover:border-[var(--color-accent)]"
							draggable={true}
							ondragstart={(e) => dragStart(item.file_id, status, e)}
							onclick={() => (onPick ? onPick(item.file_id) : goto(`/application/${item.file_id}`))}
						>
							<div class="truncate font-medium">{item.name}</div>
							<div class="truncate text-xs text-[var(--color-muted)]">{item.file_id}</div>
						</button>
					{/each}
				</div>
			</div>
		{/each}
	</div>
{/if}
