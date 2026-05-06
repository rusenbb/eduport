<script lang="ts">
	import { page } from '$app/state';
	import { getCounts, getTags } from '$lib/api/metadata';
	import { filters } from '$lib/stores/filters';
	import { status } from '$lib/stores/status';
	import { ENTITY_TYPES, type EntityType } from '$lib/types';
	import { onMount } from 'svelte';
	import Icon, { type IconName } from './Icon.svelte';

	const workspace: { href: string; label: string; icon: IconName }[] = [
		{ href: '/', label: 'Dashboard', icon: 'dashboard' },
		{ href: '/deadlines', label: 'Deadlines', icon: 'deadlines' },
		{ href: '/status', label: 'Status', icon: 'status' }
	];

	const database: { href: string; label: string; icon: IconName; type: EntityType }[] = [
		{ href: '/program', label: 'Programs', icon: 'program', type: 'program' },
		{ href: '/application', label: 'Applications', icon: 'application', type: 'application' },
		{ href: '/person', label: 'People', icon: 'person', type: 'person' },
		{ href: '/university', label: 'Universities', icon: 'university', type: 'university' },
		{ href: '/lab', label: 'Labs', icon: 'lab', type: 'lab' },
		{ href: '/document', label: 'Documents', icon: 'document', type: 'document' },
		{ href: '/email', label: 'Emails', icon: 'email', type: 'email' },
		{ href: '/note', label: 'Notes', icon: 'note', type: 'note' }
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
				class="flex items-center gap-2.5 rounded px-2 py-1.5 text-[var(--color-text)] hover:bg-white/5"
				class:active={isActive(item.href)}
			>
				<Icon name={item.icon} class="text-[var(--color-muted)]" />
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
				<span class="flex min-w-0 items-center gap-2.5">
					<Icon name={item.icon} class="text-[var(--color-muted)]" />
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
		<a href="/trash" class="flex items-center gap-2.5 rounded px-2 py-1.5 text-[var(--color-muted)] hover:bg-white/5" class:active={isActive('/trash')}>
			<Icon name="trash" /><span>Trash</span>
		</a>
		<a href="/settings" class="flex items-center gap-2.5 rounded px-2 py-1.5 text-[var(--color-muted)] hover:bg-white/5" class:active={isActive('/settings')}>
			<Icon name="settings" /><span>Settings</span>
		</a>
	</div>
</aside>

<style>
	.active {
		background-color: rgba(108, 182, 255, 0.12);
		color: #ddebff;
	}
</style>
