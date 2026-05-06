<script lang="ts">
	import { page } from '$app/state';
	import { getCounts, getTags } from '$lib/api/metadata';
	import { filters } from '$lib/stores/filters';
	import { status } from '$lib/stores/status';
	import { ENTITY_TYPES, type EntityType } from '$lib/types';
	import { onMount } from 'svelte';

	const workspace = [
		{ href: '/', label: 'Dashboard', icon: 'Dashboard' },
		{ href: '/deadlines', label: 'Deadlines', icon: 'Dates' },
		{ href: '/status', label: 'Status', icon: 'Status' }
	];

	const database: { href: string; label: string; icon: string; type: EntityType }[] = [
		{ href: '/program', label: 'Programs', icon: 'Prog', type: 'program' },
		{ href: '/application', label: 'Applications', icon: 'App', type: 'application' },
		{ href: '/person', label: 'People', icon: 'Ppl', type: 'person' },
		{ href: '/university', label: 'Universities', icon: 'Univ', type: 'university' },
		{ href: '/lab', label: 'Labs', icon: 'Lab', type: 'lab' },
		{ href: '/document', label: 'Documents', icon: 'Doc', type: 'document' },
		{ href: '/email', label: 'Emails', icon: 'Mail', type: 'email' },
		{ href: '/note', label: 'Notes', icon: 'Note', type: 'note' }
	];

	let counts: Partial<Record<EntityType, number>> = $state({});
	let tags: { tag: string; count: number }[] = $state([]);

	async function refreshMeta() {
		try {
			const [nextCounts, nextTags] = await Promise.all([getCounts(), getTags()]);
			counts = nextCounts;
			tags = nextTags.slice(0, 8);
		} catch {
			counts = {};
			tags = [];
		}
	}

	onMount(() => {
		void refreshMeta();
	});

	function isActive(href: string): boolean {
		if (href === '/') return page.url.pathname === '/';
		return page.url.pathname.startsWith(href);
	}
</script>

<aside class="flex h-full w-[220px] flex-col gap-3 border-r border-[var(--color-border)] bg-[var(--color-panel)] p-3 text-sm">
	<div class="px-2 pb-1 pt-2">
		<div class="text-sm font-semibold">Eduport</div>
		<div class="mt-1 text-[10px] text-[var(--color-muted)]">
			{$status.sidecarUp ? `${ENTITY_TYPES.length} entity types` : 'Backend offline'}
		</div>
	</div>
	<div class="flex flex-col gap-1">
		<h4 class="px-2 pt-2 text-[10px] uppercase tracking-wider text-[var(--color-muted)]">Workspace</h4>
		{#each workspace as item}
			<a
				href={item.href}
				class="flex items-center gap-2 rounded px-2 py-1.5 text-[var(--color-text)] hover:bg-white/5"
				class:active={isActive(item.href)}
			>
				<span class="w-12 text-[10px] uppercase tracking-wide text-[var(--color-muted)]">{item.icon}</span>
				<span>{item.label}</span>
			</a>
		{/each}
	</div>

	<div class="flex flex-col gap-1">
		<h4 class="px-2 pt-2 text-[10px] uppercase tracking-wider text-[var(--color-muted)]">Database</h4>
		{#each database as item}
			<a
				href={item.href}
				class="flex items-center justify-between gap-2 rounded px-2 py-1.5 text-[var(--color-text)] hover:bg-white/5"
				class:active={isActive(item.href)}
			>
				<span class="flex min-w-0 items-center gap-2">
					<span class="w-9 text-[10px] uppercase tracking-wide text-[var(--color-muted)]">{item.icon}</span>
					<span class="truncate">{item.label}</span>
				</span>
				<span class="text-[10px] text-[var(--color-muted)]">{counts[item.type] ?? 0}</span>
			</a>
		{/each}
	</div>

	<div class="flex flex-col gap-1">
		<h4 class="px-2 pt-2 text-[10px] uppercase tracking-wider text-[var(--color-muted)]">Tags</h4>
		{#if tags.length === 0}
			<div class="px-2 py-1 text-xs text-[var(--color-muted)]">No tags yet</div>
		{:else}
			{#each tags as item}
				<button
					class="flex items-center justify-between rounded px-2 py-1.5 text-left text-[var(--color-text)] hover:bg-white/5"
					class:active={$filters.tags.includes(item.tag)}
					onclick={() => filters.toggleTag(item.tag)}
				>
					<span class="truncate">#{item.tag}</span>
					<span class="text-[10px] text-[var(--color-muted)]">{item.count}</span>
				</button>
			{/each}
		{/if}
	</div>

	<div class="mt-auto flex flex-col gap-1">
		<a href="/trash" class="flex items-center gap-2 rounded px-2 py-1.5 text-[var(--color-muted)] hover:bg-white/5" class:active={isActive('/trash')}>
			<span class="w-12 text-[10px] uppercase tracking-wide">Trash</span><span>Trash</span>
		</a>
		<a href="/settings" class="flex items-center gap-2 rounded px-2 py-1.5 text-[var(--color-muted)] hover:bg-white/5" class:active={isActive('/settings')}>
			<span class="w-12 text-[10px] uppercase tracking-wide">Prefs</span><span>Settings</span>
		</a>
	</div>
</aside>

<style>
	.active {
		background-color: rgba(108, 182, 255, 0.12);
		color: #ddebff;
	}
</style>
