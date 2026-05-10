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
	import { listenCoreEvent } from '$lib/api/client';
	import { isTauri } from '$lib/tauri';
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
	let showAllTags = $state(false);

	const TAG_COLLAPSED_LIMIT = 8;

	// User-visible tags only. `eduport-type/<value>` and `eduport-doctype/<value>`
	// are internal discriminators (the file's entity type, the document subtype);
	// they appear on every entity and would dominate the sidebar.
	const visibleTags = $derived(
		tags.filter(
			(t) => !t.tag.startsWith('eduport-type/') && !t.tag.startsWith('eduport-doctype/')
		)
	);
	const displayedTags = $derived(
		showAllTags ? visibleTags : visibleTags.slice(0, TAG_COLLAPSED_LIMIT)
	);
	const hiddenTagCount = $derived(Math.max(0, visibleTags.length - TAG_COLLAPSED_LIMIT));

	async function refreshMeta() {
		try {
			const [nextCounts, nextTags] = await Promise.all([getCounts(), getTags()]);
			counts = nextCounts;
			tags = nextTags;
		} catch {
			counts = {};
			tags = [];
		}
	}

	onMount(() => {
		void refreshMeta();
		void schemaStore.load();
		if (!isTauri()) return;
		let unlisten: (() => void) | null = null;
		let pending: ReturnType<typeof setTimeout> | null = null;
		const scheduleRefresh = () => {
			if (pending) clearTimeout(pending);
			pending = setTimeout(() => {
				pending = null;
				void refreshMeta();
			}, 200);
		};
		void listenCoreEvent<{ kind: string }>('eduport:vault-event', (payload) => {
			// Counts & tag list can change on any add/remove/edit. Cheap
			// to refetch (one indexed query each) and only fires for
			// external changes — in-app writes suppress watcher events
			// via `note_self_write`, so we never thrash.
			if (
				payload.kind === 'entity_changed' ||
				payload.kind === 'entity_deleted' ||
				payload.kind === 'needs_rescan'
			) {
				scheduleRefresh();
			}
		}).then((u) => {
			unlisten = u;
		});
		return () => {
			if (pending) clearTimeout(pending);
			unlisten?.();
		};
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
		// Stale-while-revalidate. The previous version wrote
		// `propertyCounts = {}` on every effect run, which caused the
		// property-chip section to blink empty on type switches (and
		// occasionally on intra-type navigation). Now we keep the
		// previous values until the new fetch resolves; cached counts
		// for *other* types stay in the object but the template
		// iterates `chipProperties` for the current type so they're
		// inert (and become a free cache on the next visit).
		if (!t || props.length === 0) return;
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

<aside class="flex h-full w-[220px] flex-col border-r border-[var(--color-border)] bg-[var(--color-panel)] text-sm">
	<!-- Scroll region. Everything that can grow with the user's data
		 (database counts, property chips, tags) lives here; the footer
		 (Trash, Settings) is pinned below so it stays reachable at any
		 zoom level. -->
	<div class="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-3">
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
								class="flex items-center justify-between gap-2 rounded px-2 py-1.5 text-left text-[var(--color-text)] hover:bg-white/5"
								class:active={active.has(row.value)}
								onclick={() => togglePropertyFilter(prop, row.value)}
							>
								<span class="flex min-w-0 items-center gap-2 truncate">
									<span class="inline-flex h-2 w-2 flex-shrink-0 rounded-full {c.bg} border {c.border}"></span>
									<span class="truncate text-xs">{opt?.label ?? row.value}</span>
								</span>
								<span class="flex-shrink-0 text-[10px] text-[var(--color-muted)]">{row.count}</span>
							</button>
						{/each}
					</div>
				{/if}
			{/each}
		{/if}

		<div class="flex flex-col gap-1">
			<h4 class="px-2 pt-2 text-[10px] uppercase tracking-wider text-[var(--color-muted)]">Tags</h4>
			{#if visibleTags.length === 0}
				<div class="px-2 py-1 text-xs text-[var(--color-muted)]">No tags yet</div>
			{:else}
				{#each displayedTags as item (item.tag)}
					<button
						class="flex items-center justify-between gap-2 rounded px-2 py-1.5 text-left text-[var(--color-text)] hover:bg-white/5"
						class:active={$filters.tags.includes(item.tag)}
						onclick={() => filters.toggleTag(item.tag)}
					>
						<span class="min-w-0 flex-1 truncate">#{item.tag}</span>
						<span class="flex-shrink-0 text-[10px] text-[var(--color-muted)]">{item.count}</span>
					</button>
				{/each}
				{#if hiddenTagCount > 0}
					<button
						class="px-2 py-1 text-left text-[10px] uppercase tracking-wider text-[var(--color-muted)] hover:text-[var(--color-text)]"
						onclick={() => (showAllTags = !showAllTags)}
					>
						{showAllTags ? 'Show fewer' : `Show all (${hiddenTagCount} more)`}
					</button>
				{/if}
			{/if}
		</div>
	</div>

	<!-- Pinned footer: always reachable, never scrolled out. -->
	<div class="flex flex-shrink-0 flex-col gap-1 border-t border-[var(--color-border)] p-3">
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
