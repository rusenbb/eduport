<script lang="ts">
	import { goto } from '$app/navigation';
	import { getAppStatus } from '$lib/api/status';
	import { listEntities, getEntity } from '$lib/api/entities';
	import { createSampleData } from '$lib/sample-data';
	import { settings } from '$lib/stores/settings';
	import type { ApplicationStatus, EntityListItem } from '$lib/types';

	const statuses: ApplicationStatus[] = [
		'planning',
		'drafting',
		'submitted',
		'decision-pending',
		'accepted',
		'rejected',
		'withdrawn'
	];

	let pipeline = $state<Record<ApplicationStatus, number>>({
		planning: 0,
		drafting: 0,
		submitted: 0,
		'decision-pending': 0,
		accepted: 0,
		rejected: 0,
		withdrawn: 0
	});
	let upcoming: { fileId: string; name: string; deadline: string; type: string }[] = $state([]);
	let outstandingRecs: EntityListItem[] = $state([]);
	let recentEmails: { fileId: string; name: string; date: string; direction: string }[] = $state([]);
	let entityCount = $state(0);
	let seeding = $state(false);
	let loaded = $state(false);

	async function load() {
		const appStatus = await getAppStatus().catch(() => null);
		entityCount = appStatus?.entities ?? 0;
		const apps = await listEntities('application');
		const counts: Record<ApplicationStatus, number> = {
			planning: 0,
			drafting: 0,
			submitted: 0,
			'decision-pending': 0,
			accepted: 0,
			rejected: 0,
			withdrawn: 0
		};
		const upcomingList: typeof upcoming = [];

		for (const app of apps) {
			try {
				const detail = await getEntity('application', app.file_id);
				const status = (detail.entity.status as ApplicationStatus) ?? 'planning';
				counts[status] = (counts[status] ?? 0) + 1;
				const deadline = detail.entity.internal_deadline as string | null;
				if (deadline) {
					upcomingList.push({
						fileId: app.file_id,
						name: app.name,
						deadline,
						type: 'application'
					});
				}
			} catch {
				// skip
			}
		}

		// Programs deadlines
		const programs = await listEntities('program');
		for (const prog of programs) {
			try {
				const detail = await getEntity('program', prog.file_id);
				const deadline = detail.entity.deadline as string | null;
				if (deadline) {
					upcomingList.push({
						fileId: prog.file_id,
						name: prog.name,
						deadline,
						type: 'program'
					});
				}
			} catch {
				// skip
			}
		}

		const today = new Date().toISOString().slice(0, 10);
		const horizon = new Date();
		horizon.setDate(horizon.getDate() + 30);
		const horizonStr = horizon.toISOString().slice(0, 10);

		upcoming = upcomingList
			.filter((u) => u.deadline >= today && u.deadline <= horizonStr)
			.sort((a, b) => a.deadline.localeCompare(b.deadline));
		pipeline = counts;

		// Outstanding recommendations
		const docs = await listEntities('document');
		const recs: EntityListItem[] = [];
		for (const doc of docs) {
			try {
				const detail = await getEntity('document', doc.file_id);
				if (detail.entity.status === 'requested') {
					recs.push(doc);
				}
			} catch {
				// skip
			}
		}
		outstandingRecs = recs;

		const emails = await listEntities('email');
		const emailRows: typeof recentEmails = [];
		for (const email of emails) {
			try {
				const detail = await getEntity('email', email.file_id);
				emailRows.push({
					fileId: email.file_id,
					name: email.name,
					date: (detail.entity.date as string | null) ?? '',
					direction: (detail.entity.direction as string | null) ?? ''
				});
			} catch {
				// skip
			}
		}
		recentEmails = emailRows.sort((a, b) => b.date.localeCompare(a.date)).slice(0, 5);
		loaded = true;
	}

	async function seed() {
		seeding = true;
		try {
			await createSampleData();
			await load();
		} finally {
			seeding = false;
		}
	}

	$effect(() => {
		void load();
	});
</script>

<main class="p-6">
	<header class="mb-6">
		<h1 class="text-2xl font-semibold">Dashboard</h1>
		{#if $settings}
			<p class="mt-1 text-xs text-[var(--color-muted)]">Data folder: <code>{$settings.data_folder}</code></p>
		{/if}
	</header>

	{#if !loaded}
		<div class="text-center text-[var(--color-muted)]">Loading…</div>
	{:else if entityCount === 0}
		<section class="border-y border-[var(--color-border)] py-10 text-center">
			<h2 class="text-lg font-semibold">Start with a small sample set</h2>
			<p class="mx-auto mt-2 max-w-md text-sm text-[var(--color-muted)]">
				Create one university, one program, and one application to see links, deadlines, and checkboxes in context.
			</p>
			<button
				class="mt-5 rounded border border-[var(--color-accent)] bg-[var(--color-accent)]/15 px-4 py-2 text-sm font-medium text-[var(--color-accent)] hover:bg-[var(--color-accent)]/25 disabled:opacity-50"
				disabled={seeding}
				onclick={seed}
			>
				{seeding ? 'Creating...' : 'Create sample data'}
			</button>
		</section>
	{:else}
		<section class="mb-8">
			<h2 class="mb-3 text-sm font-medium uppercase tracking-wider text-[var(--color-muted)]">Pipeline</h2>
			<div class="grid grid-cols-7 gap-2 text-center text-xs">
				{#each statuses as status}
					<button
						class="rounded border border-[var(--color-border)] bg-[var(--color-panel)] p-3 hover:border-[var(--color-accent)]"
						onclick={() => goto('/application')}
					>
						<div class="text-xl font-semibold">{pipeline[status]}</div>
						<div class="mt-1 capitalize text-[var(--color-muted)]">{status}</div>
					</button>
				{/each}
			</div>
		</section>

		<section class="mb-8">
			<h2 class="mb-3 text-sm font-medium uppercase tracking-wider text-[var(--color-muted)]">
				Upcoming deadlines (next 30 days)
			</h2>
			{#if upcoming.length === 0}
				<p class="text-xs text-[var(--color-muted)]">Nothing in the next 30 days.</p>
			{:else}
				<ul class="rounded border border-[var(--color-border)] bg-[var(--color-panel)]">
					{#each upcoming as item}
						<li>
							<button
								class="flex w-full items-center justify-between border-b border-[var(--color-border)] px-3 py-2 text-left text-sm last:border-b-0 hover:bg-white/5"
								onclick={() => goto(`/${item.type}/${item.fileId}`)}
							>
								<span class="truncate">{item.name}</span>
								<span class="ml-3 flex-shrink-0 font-mono text-xs text-[var(--color-warn)]">{item.deadline}</span>
							</button>
						</li>
					{/each}
				</ul>
			{/if}
		</section>

		<section class="mb-8">
			<h2 class="mb-3 text-sm font-medium uppercase tracking-wider text-[var(--color-muted)]">
				Outstanding recommendations
			</h2>
			{#if outstandingRecs.length === 0}
				<p class="text-xs text-[var(--color-muted)]">None requested or all received.</p>
			{:else}
				<ul class="rounded border border-[var(--color-border)] bg-[var(--color-panel)]">
					{#each outstandingRecs as rec}
						<li>
							<button
								class="block w-full border-b border-[var(--color-border)] px-3 py-2 text-left text-sm last:border-b-0 hover:bg-white/5"
								onclick={() => goto(`/document/${rec.file_id}`)}
							>
								{rec.name}
							</button>
						</li>
					{/each}
				</ul>
			{/if}
		</section>

		<section>
			<h2 class="mb-3 text-sm font-medium uppercase tracking-wider text-[var(--color-muted)]">Recent emails</h2>
			{#if recentEmails.length === 0}
				<p class="text-xs text-[var(--color-muted)]">No emails logged yet.</p>
			{:else}
				<ul class="rounded border border-[var(--color-border)] bg-[var(--color-panel)]">
					{#each recentEmails as email}
						<li>
							<button
								class="flex w-full items-center justify-between border-b border-[var(--color-border)] px-3 py-2 text-left text-sm last:border-b-0 hover:bg-white/5"
								onclick={() => goto(`/email/${email.fileId}`)}
							>
								<span class="truncate">{email.name}</span>
								<span class="ml-3 flex-shrink-0 text-xs text-[var(--color-muted)]">{email.date} · {email.direction}</span>
							</button>
						</li>
					{/each}
				</ul>
			{/if}
		</section>
	{/if}
</main>
