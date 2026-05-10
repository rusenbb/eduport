<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { getCounts, getTags } from '$lib/api/metadata';
	import {
		getPropertyCounts,
		parseFilterParams,
		writeFilterParams
	} from '$lib/api/schema';
	import { filters } from '$lib/stores/filters';
	import { schemaStore } from '$lib/stores/schema';
	import { status } from '$lib/stores/status';
	import { ENTITY_TYPES, type EntityType } from '$lib/types';
	import {
		COLOR_CLASSES,
		type Property,
		type PropertyCount,
		type SingleSelectProperty,
		type MultiSelectProperty
	} from '$lib/types/schema';
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
	let propertyCounts: Record<string, PropertyCount[]> = $state({});

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
		void schemaStore.load();
	});

	function isActive(href: string): boolean {
		if (href === '/') return page.url.pathname === '/';
		return page.url.pathname.startsWith(href);
	}

	function pathExact(href: string): boolean {
		// /settings should not light up when on /settings/schema (the schema link
		// has its own entry above).
		return page.url.pathname === href;
	}

	// Detect which entity type's list view we're on (e.g. /university or
	// /university/some-id). Returns null when not on an entity-type route.
	const currentEntityType = $derived.by((): EntityType | null => {
		const segments = page.url.pathname.split('/').filter(Boolean);
		if (segments.length === 0) return null;
		const first = segments[0];
		return (ENTITY_TYPES as string[]).includes(first) ? (first as EntityType) : null;
	});

	const chipProperties = $derived.by((): (SingleSelectProperty | MultiSelectProperty)[] => {
		if (!currentEntityType) return [];
		const props = $schemaStore.schema?.types[currentEntityType]?.properties ?? [];
		return props.filter(
			(p): p is SingleSelectProperty | MultiSelectProperty =>
				p.type === 'single-select' || p.type === 'multi-select'
		);
	});

	$effect(() => {
		const t = currentEntityType;
		const props = chipProperties;
		propertyCounts = {};
		if (!t) return;
		for (const p of props) {
			void getPropertyCounts(t, p.key)
				.then((r) => {
					propertyCounts = { ...propertyCounts, [p.key]: r.values };
				})
				.catch(() => {
					/* ignore */
				});
		}
	});

	function activeFilterValuesFor(key: string): Set<string> {
		const filtersFromUrl = parseFilterParams(page.url.searchParams);
		const v = filtersFromUrl.text[key];
		return v ? new Set([v]) : new Set();
	}

	function togglePropertyFilter(prop: Property, value: string) {
		if (!currentEntityType) return;
		const url = new URL(page.url);
		const filters = parseFilterParams(url.searchParams);
		const current = filters.text[prop.key];
		if (current === value) {
			delete filters.text[prop.key];
		} else {
			filters.text[prop.key] = value;
		}
		writeFilterParams(url.searchParams, filters);
		// If we're not currently on this entity type's list, navigate there.
		const targetPath = `/${currentEntityType}`;
		if (page.url.pathname !== targetPath) {
			url.pathname = targetPath;
		}
		void goto(url, { replaceState: false, keepFocus: false });
	}

	function optionFor(prop: Property, value: string): { label: string; color: string } | null {
		if (prop.type !== 'single-select' && prop.type !== 'multi-select') return null;
		const opt = prop.options.find((o) => o.value === value);
		return opt ? { label: opt.label, color: opt.color } : null;
	}
</script>

<aside class="flex h-full w-[220px] flex-col gap-3 border-r border-[var(--color-border)] bg-[var(--color-panel)] p-3 text-sm">
	<div class="px-2 pb-1 pt-2">
		<div class="text-sm font-semibold">Eduport</div>
		<div class="mt-1 text-[10px] text-[var(--color-muted)]">
			{$status.coreUp ? `${ENTITY_TYPES.length} entity types` : 'Backend offline'}
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
		<!-- Schema link sits under Database — visually distinct (smaller text,
			 muted color, italic) so it doesn't read as a 9th entity type. -->
		<a
			href="/settings/schema"
			class="ml-1 mt-0.5 flex items-center gap-2 rounded px-2 py-1 text-[11px] italic text-[var(--color-muted)] hover:bg-white/5 hover:text-[var(--color-text)]"
			class:active={isActive('/settings/schema')}
		>
			<Icon name="sliders" size={12} />
			<span>Customize fields…</span>
		</a>
	</div>

	{#if chipProperties.length > 0}
		{#each chipProperties as prop (prop.key)}
			{@const counts = propertyCounts[prop.key] ?? []}
			{@const active = activeFilterValuesFor(prop.key)}
			{#if counts.length > 0}
				<div class="flex flex-col gap-1">
					<h4 class="px-2 pt-2 text-[10px] uppercase tracking-wider text-[var(--color-muted)]">
						{prop.name}
					</h4>
					{#each counts as row (row.value)}
						{@const opt = optionFor(prop, row.value)}
						{@const colorKey = (opt?.color ?? 'gray') as keyof typeof COLOR_CLASSES}
						{@const c = COLOR_CLASSES[colorKey]}
						<button
							class="flex items-center justify-between rounded px-2 py-1.5 text-left text-[var(--color-text)] hover:bg-white/5"
							class:active={active.has(row.value)}
							onclick={() => togglePropertyFilter(prop, row.value)}
						>
							<span class="flex items-center gap-2 truncate">
								<span class="inline-flex h-2 w-2 rounded-full {c.bg} border {c.border}"></span>
								<span class="truncate text-xs">{opt?.label ?? row.value}</span>
							</span>
							<span class="text-[10px] text-[var(--color-muted)]">{row.count}</span>
						</button>
					{/each}
				</div>
			{/if}
		{/each}
	{/if}

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
		<a href="/settings" class="flex items-center gap-2.5 rounded px-2 py-1.5 text-[var(--color-muted)] hover:bg-white/5" class:active={pathExact('/settings')}>
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
