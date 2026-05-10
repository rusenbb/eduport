<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { purgeOrphans } from '$lib/api/schema';
	import {
		BUILTIN_KIND_LABELS,
		FIELD_DEFS,
		TYPE_LABELS,
		builtinKindToPropertyType
	} from '$lib/entities/meta';
	import Icon from '$lib/components/Icon.svelte';
	import PropertyTypeIcon from '$lib/components/properties/PropertyTypeIcon.svelte';
	import { schemaStore } from '$lib/stores/schema';
	import { ENTITY_TYPES, type EntityType } from '$lib/types';
	import { COLOR_PALETTE, type Property, type PropertyType, type SelectOption } from '$lib/types/schema';
	import { onMount } from 'svelte';

	const PROPERTY_TYPE_LABELS: Record<PropertyType, string> = {
		text: 'Text',
		number: 'Number',
		date: 'Date',
		checkbox: 'Checkbox',
		'single-select': 'Single select',
		'multi-select': 'Multi-select',
		url: 'URL',
		relation: 'Relation'
	};

	const PROPERTY_TYPES: PropertyType[] = [
		'text',
		'number',
		'date',
		'checkbox',
		'single-select',
		'multi-select',
		'url',
		'relation'
	];

	const TIER_TEMPLATE_TYPES: EntityType[] = ['university', 'program', 'application'];

	let activeType: EntityType = $state(
		(page.url.searchParams.get('type') as EntityType) || 'university'
	);
	let editing: { mode: 'add' } | { mode: 'edit'; property: Property } | null = $state(null);
	let templateBusy = $state(false);
	let templateMessage: string | null = $state(null);
	let topError: string | null = $state(null);

	onMount(() => {
		void schemaStore.load();
	});

	function setActive(type: EntityType) {
		activeType = type;
		const url = new URL(page.url);
		url.searchParams.set('type', type);
		void goto(url, { replaceState: true, keepFocus: true, noScroll: true });
		editing = null;
		templateMessage = null;
		topError = null;
	}

	async function applyTier() {
		templateBusy = true;
		templateMessage = null;
		topError = null;
		try {
			const { added, existed } = await schemaStore.applyTierTemplate(TIER_TEMPLATE_TYPES);
			templateMessage =
				`Added tier on: ${added.length === 0 ? 'none' : added.join(', ')}` +
				(existed.length > 0 ? `; already present on: ${existed.join(', ')}` : '');
		} catch (e) {
			topError = e instanceof Error ? e.message : String(e);
		} finally {
			templateBusy = false;
		}
	}

	async function deleteProperty(prop: Property) {
		if (!confirm(`Delete "${prop.name}" from ${TYPE_LABELS[activeType]}? Existing values become orphaned.`)) return;
		try {
			await schemaStore.deleteProperty(activeType, prop.key);
		} catch (e) {
			topError = e instanceof Error ? e.message : String(e);
		}
	}

	// Drag-reorder for the property list. Uses HTML5 drag API directly so we
	// don't take on a third-party drag library; the trade-off is rougher
	// drop animations than Notion's. Reorder commits via schemaStore as soon
	// as the drop lands.
	let dragSourceIdx: number | null = $state(null);
	let dragOverIdx: number | null = $state(null);

	function onDragStart(e: DragEvent, idx: number) {
		dragSourceIdx = idx;
		if (e.dataTransfer) {
			e.dataTransfer.effectAllowed = 'move';
			e.dataTransfer.setData('text/plain', String(idx));
		}
	}

	function onDragOver(e: DragEvent, idx: number) {
		if (dragSourceIdx === null || dragSourceIdx === idx) return;
		e.preventDefault();
		if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
		dragOverIdx = idx;
	}

	async function onDrop(e: DragEvent, dropIdx: number) {
		e.preventDefault();
		const fromIdx = dragSourceIdx;
		dragSourceIdx = null;
		dragOverIdx = null;
		if (fromIdx === null || fromIdx === dropIdx) return;
		const props = $schemaStore.schema?.types[activeType]?.properties;
		if (!props) return;
		const reordered = [...props];
		const [moved] = reordered.splice(fromIdx, 1);
		reordered.splice(dropIdx, 0, moved);
		try {
			await schemaStore.reorderProperties(
				activeType,
				reordered.map((p) => p.key)
			);
		} catch (err) {
			topError = err instanceof Error ? err.message : String(err);
		}
	}

	async function purge(prop: Property) {
		if (!confirm(`Permanently strip "${prop.key}" from all entity files? This cannot be undone.`)) return;
		try {
			const r = await purgeOrphans(activeType, prop.key);
			templateMessage = `Purged ${r.rewritten} files; ${r.skipped.length} skipped`;
		} catch (e) {
			topError = e instanceof Error ? e.message : String(e);
		}
	}
</script>

<main class="mx-auto flex max-w-4xl flex-col gap-6 p-6">
	<header class="flex items-center justify-between">
		<div>
			<h1 class="text-2xl font-semibold">Schema</h1>
			<p class="mt-1 text-sm text-[var(--color-muted)]">
				Define custom fields per entity type. Built-in fields are read-only.
			</p>
		</div>
		<a
			href="/settings"
			class="rounded border border-[var(--color-border)] px-3 py-1.5 text-xs hover:bg-white/5"
		>
			← Settings
		</a>
	</header>

	{#if $schemaStore.error}
		<div class="rounded border border-[var(--color-bad)] bg-red-900/20 px-3 py-2 text-sm text-[var(--color-bad)]">
			{$schemaStore.error}
		</div>
	{/if}

	<section class="flex flex-wrap items-center gap-3 border-y border-[var(--color-border)] py-4">
		<div class="flex flex-col">
			<span class="text-xs font-medium">Add tier template</span>
			<span class="text-[10px] text-[var(--color-muted)]">
				Adds a "Tier" single-select on University, Program, and Application.
			</span>
		</div>
		<button
			class="rounded border border-blue-700 bg-blue-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-blue-700 disabled:opacity-50"
			disabled={templateBusy}
			onclick={applyTier}
		>
			{templateBusy ? 'Adding…' : '+ Add tier'}
		</button>
		{#if templateMessage}
			<span class="text-xs text-[var(--color-muted)]">{templateMessage}</span>
		{/if}
	</section>

	<nav class="flex flex-wrap gap-1 border-b border-[var(--color-border)]">
		{#each ENTITY_TYPES as t}
			<button
				class="rounded-t border border-b-0 border-transparent px-3 py-1.5 text-sm hover:bg-white/5"
				class:active={t === activeType}
				onclick={() => setActive(t)}
			>
				{TYPE_LABELS[t]}
			</button>
		{/each}
	</nav>

	{#if $schemaStore.loading && !$schemaStore.schema}
		<div class="text-center text-sm text-[var(--color-muted)]">Loading schema…</div>
	{:else if $schemaStore.schema}
		{@const typeSchema = $schemaStore.schema.types[activeType]}
		{@const builtinDefs = FIELD_DEFS[activeType] ?? []}
		<section class="flex flex-col gap-3">
			<header class="flex items-center justify-between">
				<h2 class="text-sm font-semibold">Built-in fields</h2>
			</header>
			{#if builtinDefs.length === 0}
				<p class="text-xs text-[var(--color-muted)]">No structured built-in fields on this entity type.</p>
			{:else}
				<div class="grid gap-2">
					{#each builtinDefs as def}
						<div class="flex items-start gap-3 rounded border border-[var(--color-border)] bg-white/[0.015] px-3 py-2">
							<PropertyTypeIcon type={builtinKindToPropertyType(def.kind)} class="mt-0.5 text-[var(--color-muted)]" />
							<div class="min-w-0 flex-1">
								<div class="flex flex-wrap items-center gap-2">
									<span class="text-sm font-medium">{def.label}</span>
									<span class="text-[10px] text-[var(--color-muted)]">{def.key}</span>
									<span class="rounded bg-white/5 px-1.5 py-0.5 text-[10px] text-[var(--color-muted)]">
										{BUILTIN_KIND_LABELS[def.kind]}
									</span>
									<span class="rounded bg-[var(--color-muted)]/20 px-1.5 py-0.5 text-[10px] text-[var(--color-muted)]">
										built-in
									</span>
								</div>
								{#if def.kind === 'select' && def.options}
									<div class="mt-1 flex flex-wrap gap-1">
										{#each def.options as opt}
											<span class="rounded bg-white/5 px-1.5 py-0.5 text-[10px]">{opt}</span>
										{/each}
									</div>
								{/if}
								{#if def.kind === 'wikilink' && def.linkType}
									<div class="mt-1 text-[10px] text-[var(--color-muted)]">→ {def.linkType}</div>
								{/if}
								{#if def.kind === 'wikilinks' && def.linkType}
									<div class="mt-1 text-[10px] text-[var(--color-muted)]">→ {def.linkType}s</div>
								{/if}
							</div>
						</div>
					{/each}
				</div>
			{/if}
			<p class="text-[10px] text-[var(--color-muted)]">
				Built-in fields cannot be renamed or removed. Custom fields below cannot use these keys.
			</p>
		</section>

		<section class="flex flex-col gap-3 border-t border-[var(--color-border)] pt-5">
			<header class="flex items-center justify-between">
				<h2 class="text-sm font-semibold">Custom fields</h2>
				<button
					class="rounded border border-[var(--color-border)] bg-white/5 px-3 py-1 text-xs hover:bg-white/10"
					onclick={() => (editing = { mode: 'add' })}
				>
					<Icon name="plus" /> Add property
				</button>
			</header>

			{#if typeSchema.properties.length === 0}
				<div class="rounded border border-dashed border-[var(--color-border)] p-6 text-center text-sm text-[var(--color-muted)]">
					No custom fields yet. Add one to start.
				</div>
			{:else}
				<div class="grid gap-2">
					{#each typeSchema.properties as prop, idx}
						<div
							class="flex items-start gap-3 rounded border border-[var(--color-border)] bg-white/[0.03] px-3 py-2"
							class:drag-over={dragOverIdx === idx}
							role="row"
							tabindex="0"
							draggable="true"
							ondragstart={(e) => onDragStart(e, idx)}
							ondragover={(e) => onDragOver(e, idx)}
							ondragleave={() => (dragOverIdx = null)}
							ondrop={(e) => onDrop(e, idx)}
						>
							<span class="cursor-grab pr-1 pt-0.5 text-[var(--color-muted)] hover:text-[var(--color-text)]" title="Drag to reorder">⋮⋮</span>
							<div class="min-w-0 flex-1">
								<div class="flex flex-wrap items-center gap-2">
									<PropertyTypeIcon type={prop.type} class="text-[var(--color-muted)]" />
									<span class="text-sm font-medium">{prop.name}</span>
									<span class="text-[10px] text-[var(--color-muted)]">{prop.key}</span>
									<span class="rounded bg-white/5 px-1.5 py-0.5 text-[10px] text-[var(--color-muted)]">
										{PROPERTY_TYPE_LABELS[prop.type]}
									</span>
									{#if prop.required}
										<span class="rounded bg-blue-500/20 px-1.5 py-0.5 text-[10px] text-blue-200">required</span>
									{/if}
								</div>
								{#if prop.description}
									<p class="mt-0.5 text-xs text-[var(--color-muted)]">{prop.description}</p>
								{/if}
								{#if prop.type === 'single-select' || prop.type === 'multi-select'}
									<div class="mt-1 flex flex-wrap gap-1">
										{#each prop.options as opt}
											<span class="rounded bg-white/5 px-1.5 py-0.5 text-[10px]">{opt.label}</span>
										{/each}
									</div>
								{/if}
							</div>
							<div class="flex flex-shrink-0 gap-1">
								<button
									class="rounded border border-[var(--color-border)] px-2 py-1 text-[10px] hover:bg-white/5"
									onclick={() => (editing = { mode: 'edit', property: prop })}
								>
									Edit
								</button>
								<button
									class="rounded border border-[var(--color-border)] px-2 py-1 text-[10px] hover:bg-red-900/30"
									onclick={() => deleteProperty(prop)}
								>
									Delete
								</button>
							</div>
						</div>
					{/each}
				</div>
			{/if}

			<details class="text-xs">
				<summary class="cursor-pointer text-[var(--color-muted)]">Orphan cleanup</summary>
				<div class="mt-2 grid gap-1.5 text-[var(--color-muted)]">
					Strip an orphaned key from every entity file of this type. Runs only if the key is no longer in the schema.
					{#each typeSchema.properties as prop}
						<button
							class="text-left text-[var(--color-muted)] underline hover:text-[var(--color-text)]"
							onclick={() => purge(prop)}
						>
							Purge "{prop.key}" (currently declared — disabled)
						</button>
					{/each}
				</div>
			</details>
		</section>
	{/if}

	{#if topError}
		<div class="rounded border border-[var(--color-bad)] bg-red-900/20 px-3 py-2 text-sm text-[var(--color-bad)]">
			{topError}
		</div>
	{/if}
</main>

{#if editing}
	{#await import('$lib/components/properties/PropertyConfigDialog.svelte') then mod}
		{@const Dialog = mod.default}
		<Dialog
			entityType={activeType}
			mode={editing.mode}
			existing={editing.mode === 'edit' ? editing.property : null}
			onCancel={() => (editing = null)}
			onSaved={() => (editing = null)}
		/>
	{/await}
{/if}

<style>
	.active {
		background-color: rgba(108, 182, 255, 0.12);
		color: var(--color-accent);
		border-color: var(--color-border);
	}
	.drag-over {
		border-color: var(--color-accent);
		background-color: rgba(108, 182, 255, 0.1);
	}
</style>
