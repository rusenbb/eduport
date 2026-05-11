<script lang="ts">
	import { goto } from '$app/navigation';
	import { listEntities, getEntity } from '$lib/api/entities';
	import { listenCoreEvent } from '$lib/api/client';
	import { isTauri } from '$lib/tauri';
	import { onMount } from 'svelte';

	interface DeadlineRow {
		fileId: string;
		type: string;
		name: string;
		deadline: string;
		field: string;
	}

	let rows: DeadlineRow[] = $state([]);
	let loaded = $state(false);

	async function load() {
		const out: DeadlineRow[] = [];

		const programs = await listEntities('program');
		for (const p of programs) {
			const d = await getEntity('program', p.file_id).catch(() => null);
			if (!d) continue;
			const dl = d.entity.deadline as string | null;
			if (dl) out.push({ fileId: p.file_id, type: 'program', name: p.name, deadline: dl, field: 'deadline' });
		}

		const apps = await listEntities('application');
		for (const a of apps) {
			const d = await getEntity('application', a.file_id).catch(() => null);
			if (!d) continue;
			const dl = d.entity.internal_deadline as string | null;
			if (dl) {
				out.push({
					fileId: a.file_id,
					type: 'application',
					name: a.name,
					deadline: dl,
					field: 'internal deadline'
				});
			}
			// Also walk the body for inline checkboxes with dates
			const body = d.body ?? '';
			body.split('\n').forEach((line) => {
				const cb = /^- \[ \] (.+)$/.exec(line);
				if (cb) {
					const date = /(\d{4}-\d{2}-\d{2})/.exec(cb[1]);
					if (date) {
						out.push({
							fileId: a.file_id,
							type: 'application',
							name: `${a.name} — ${cb[1]}`,
							deadline: date[1],
							field: 'task'
						});
					}
				}
			});
		}

		out.sort((x, y) => x.deadline.localeCompare(y.deadline));
		rows = out;
		loaded = true;
	}

	$effect(() => {
		void load();
	});

	// Auto-refresh when external edits land — same pattern used in
	// EntityWorkspace and Sidebar.
	onMount(() => {
		if (!isTauri()) return;
		let unlisten: (() => void) | null = null;
		let pending: ReturnType<typeof setTimeout> | null = null;
		void listenCoreEvent<{ kind: string }>('eduport:vault-event', (payload) => {
			if (
				payload.kind === 'entity_changed' ||
				payload.kind === 'entity_deleted' ||
				payload.kind === 'needs_rescan'
			) {
				if (pending) clearTimeout(pending);
				pending = setTimeout(() => {
					pending = null;
					void load();
				}, 200);
			}
		}).then((u) => {
			unlisten = u;
		});
		return () => {
			if (pending) clearTimeout(pending);
			unlisten?.();
		};
	});

	const today = (() => new Date().toISOString().slice(0, 10))();

	function daysUntil(dateStr: string): number {
		const a = new Date(today + 'T00:00:00').getTime();
		const b = new Date(dateStr + 'T00:00:00').getTime();
		return Math.round((b - a) / 86_400_000);
	}

	function relativeLabel(dateStr: string): string {
		const d = daysUntil(dateStr);
		if (d < 0) return `${-d} day${-d === 1 ? '' : 's'} overdue`;
		if (d === 0) return 'Today';
		if (d === 1) return 'Tomorrow';
		if (d < 7) return `in ${d} days`;
		if (d < 30) return `in ${Math.round(d / 7)} weeks`;
		if (d < 365) return `in ${Math.round(d / 30)} months`;
		return `in ${Math.round(d / 365)} years`;
	}

	function urgencyClass(dateStr: string): string {
		const d = daysUntil(dateStr);
		if (d < 0) return 'urgency-overdue';
		if (d === 0) return 'urgency-today';
		if (d < 7) return 'urgency-soon';
		if (d < 30) return 'urgency-near';
		return 'urgency-far';
	}
</script>

<main class="p-6">
	<header class="mb-4 flex items-start justify-between gap-4">
		<div>
			<h1 class="text-2xl font-semibold">Deadlines</h1>
			<p class="mt-1 text-xs text-[var(--color-muted)]">All program deadlines, internal deadlines, and inline task dates across the vault.</p>
		</div>
		{#if loaded && rows.length > 0}
			<span class="text-xs text-[var(--color-muted)]">
				{rows.length} item{rows.length === 1 ? '' : 's'}
			</span>
		{/if}
	</header>

	{#if !loaded}
		<div class="text-center text-[var(--color-muted)]">Loading…</div>
	{:else if rows.length === 0}
		<p class="text-[var(--color-muted)]">No deadlines yet. Add a deadline to a program or an internal deadline to an application.</p>
	{:else}
		<table class="w-full text-sm">
			<thead class="border-b border-[var(--color-border)] text-[10px] uppercase tracking-wider text-[var(--color-muted)]">
				<tr>
					<th class="px-3 py-2 text-left">When</th>
					<th class="px-3 py-2 text-left">Date</th>
					<th class="px-3 py-2 text-left">Item</th>
					<th class="px-3 py-2 text-left">Field</th>
				</tr>
			</thead>
			<tbody>
				{#each rows as row}
					<tr class="border-b border-[var(--color-border)] hover:bg-white/5">
						<td class="px-3 py-2 text-xs">
							<span class={urgencyClass(row.deadline)}>{relativeLabel(row.deadline)}</span>
						</td>
						<td class="px-3 py-2 font-mono text-xs text-[var(--color-muted)]">{row.deadline}</td>
						<td class="px-3 py-2">
							<button class="text-left hover:text-[var(--color-accent)]" onclick={() => goto(`/${row.type}/${row.fileId}`)}>
								{row.name}
							</button>
						</td>
						<td class="px-3 py-2 text-xs text-[var(--color-muted)]">{row.field}</td>
					</tr>
				{/each}
			</tbody>
		</table>
	{/if}
</main>

<style>
	.urgency-overdue {
		color: var(--color-bad);
		font-weight: 600;
	}
	.urgency-today {
		color: var(--color-bad);
		font-weight: 600;
	}
	.urgency-soon {
		color: var(--color-warn);
		font-weight: 500;
	}
	.urgency-near {
		color: var(--color-warn);
	}
	.urgency-far {
		color: var(--color-muted);
	}
</style>
