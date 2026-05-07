<script lang="ts">
	import { goto } from '$app/navigation';
	import { getEntity } from '$lib/api/entities';
	import EntityRow from './EntityRow.svelte';
	import type { EntityDetail, EntityListItem } from '$lib/types';

	const UNFILED = '(no application)';

	let { items }: { items: EntityListItem[] } = $props();

	let groups: { key: string; label: string; rows: { item: EntityListItem; detail: EntityDetail }[] }[] = $state([]);
	let loading = $state(true);

	async function load() {
		loading = true;
		// N+1 by design — same pattern as KanbanBoard.load() / Dashboard.
		// Emails carry related_application as a wikilink in frontmatter, which the
		// list endpoint doesn't surface; we hydrate per-item to group by it.
		const enriched = await Promise.all(
			items.map(async (item) => {
				try {
					const detail = await getEntity('email', item.file_id);
					return { item, detail };
				} catch {
					return null;
				}
			})
		);
		const valid = enriched.filter((row): row is { item: EntityListItem; detail: EntityDetail } => !!row);

		const buckets = new Map<string, { item: EntityListItem; detail: EntityDetail }[]>();
		for (const row of valid) {
			const link = (row.detail.entity.related_application as string | null) ?? '';
			const key = link.length > 0 ? link : UNFILED;
			if (!buckets.has(key)) buckets.set(key, []);
			buckets.get(key)!.push(row);
		}

		groups = Array.from(buckets.entries())
			.map(([key, rows]) => ({
				key,
				// Strip the wikilink syntax `[[…]]` for the header label.
				label: key === UNFILED ? UNFILED : key.replace(/^\[\[|\]\]$/g, ''),
				rows: rows.sort((a, b) => {
					const da = (a.detail.entity.date as string | null) ?? '';
					const db = (b.detail.entity.date as string | null) ?? '';
					return db.localeCompare(da); // newest first within group
				})
			}))
			.sort((a, b) => {
				if (a.key === UNFILED) return 1;
				if (b.key === UNFILED) return -1;
				return a.label.localeCompare(b.label);
			});

		loading = false;
	}

	$effect(() => {
		// Re-run when item set changes.
		items;
		void load();
	});
</script>

{#if loading}
	<div class="p-8 text-center text-[var(--color-muted)]">Loading…</div>
{:else if groups.length === 0}
	<div class="p-8 text-center text-[var(--color-muted)]">No emails yet.</div>
{:else}
	<div class="flex flex-col">
		{#each groups as group (group.key)}
			<div class="border-b border-[var(--color-border)]">
				<header class="flex items-center justify-between bg-[var(--color-panel)] px-3 py-2 text-[10px] uppercase tracking-wider text-[var(--color-muted)]">
					<span class="truncate">{group.label}</span>
					<span>{group.rows.length}</span>
				</header>
				{#each group.rows as { item, detail } (item.file_id)}
					<EntityRow
						{item}
						type="email"
						summary={(detail.entity.date as string | null) ?? ''}
						onclick={() => goto(`/email/${item.file_id}`)}
					/>
				{/each}
			</div>
		{/each}
	</div>
{/if}
