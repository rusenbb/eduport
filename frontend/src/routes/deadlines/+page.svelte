<script lang="ts">
	import { goto } from '$app/navigation';
	import { listEntities, getEntity } from '$lib/api/entities';

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
			const d = await getEntity('program', p.file_id);
			const dl = d.entity.deadline as string | null;
			if (dl) out.push({ fileId: p.file_id, type: 'program', name: p.name, deadline: dl, field: 'deadline' });
		}

		const apps = await listEntities('application');
		for (const a of apps) {
			const d = await getEntity('application', a.file_id);
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
</script>

<main class="p-6">
	<header class="mb-4">
		<h1 class="text-2xl font-semibold">Deadlines</h1>
		<p class="mt-1 text-xs text-[var(--color-muted)]">All program deadlines, internal deadlines, and inline task dates across the vault.</p>
	</header>

	{#if !loaded}
		<div class="text-center text-[var(--color-muted)]">Loading…</div>
	{:else if rows.length === 0}
		<p class="text-[var(--color-muted)]">No deadlines yet.</p>
	{:else}
		<table class="w-full text-sm">
			<thead class="border-b border-[var(--color-border)] text-[10px] uppercase tracking-wider text-[var(--color-muted)]">
				<tr>
					<th class="px-3 py-2 text-left">Date</th>
					<th class="px-3 py-2 text-left">Item</th>
					<th class="px-3 py-2 text-left">Field</th>
				</tr>
			</thead>
			<tbody>
				{#each rows as row}
					<tr class="border-b border-[var(--color-border)] hover:bg-white/5">
						<td class="px-3 py-2 font-mono text-xs text-[var(--color-warn)]">{row.deadline}</td>
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
