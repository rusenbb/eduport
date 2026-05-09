<script lang="ts">
	/**
	 * View tabs across the top of every entity list view. Includes a
	 * built-in "All" tab (no filter applied) followed by user-saved views,
	 * a "+ New view" button, and a per-tab menu (Rename / Delete).
	 */
	import { viewsStore } from '$lib/stores/views';
	import type { EntityType } from '$lib/types';
	import type { View } from '$lib/types/view';
	import Icon from './Icon.svelte';

	let {
		entityType,
		activeViewId,
		onSelect,
		onSaveCurrent,
		onActiveDeleted
	}: {
		entityType: EntityType;
		activeViewId: string | null;
		onSelect: (view: View | null) => void;
		onSaveCurrent: () => void;
		onActiveDeleted: () => void;
	} = $props();

	const views = $derived($viewsStore.file?.types[entityType]?.views ?? []);

	let menuOpenFor: string | null = $state(null);
	let renaming: View | null = $state(null);

	async function deleteView(v: View) {
		if (!confirm(`Delete view "${v.name}"?`)) return;
		menuOpenFor = null;
		const wasActive = v.id === activeViewId;
		await viewsStore.delete(entityType, v.id);
		if (wasActive) onActiveDeleted();
	}
</script>

<div class="flex items-center gap-1 overflow-x-auto border-b border-[var(--color-border)] px-2 pt-2 text-xs">
	<button
		class="flex items-center gap-1 whitespace-nowrap rounded-t border border-b-0 border-transparent px-3 py-1.5 hover:bg-white/5"
		class:active={activeViewId === null}
		onclick={() => onSelect(null)}
	>
		All
	</button>
	{#each views as v (v.id)}
		<div class="relative flex items-stretch">
			<button
				class="flex items-center gap-1 whitespace-nowrap rounded-t border border-b-0 border-transparent px-3 py-1.5 hover:bg-white/5"
				class:active={activeViewId === v.id}
				onclick={() => onSelect(v)}
			>
				{v.name}
			</button>
			{#if activeViewId === v.id}
				<button
					class="rounded-t border-b-0 px-1 py-1.5 text-[var(--color-muted)] hover:bg-white/5 hover:text-[var(--color-text)]"
					aria-label="View options"
					onclick={() => (menuOpenFor = menuOpenFor === v.id ? null : v.id)}
				>
					⋯
				</button>
				{#if menuOpenFor === v.id}
					<div class="absolute right-0 top-full z-30 w-36 overflow-hidden rounded border border-[var(--color-border)] bg-[var(--color-panel)] shadow-xl">
						<button
							class="block w-full border-b border-[var(--color-border)] px-3 py-1.5 text-left hover:bg-white/5"
							onclick={() => {
								renaming = v;
								menuOpenFor = null;
							}}
						>
							Rename
						</button>
						<button
							class="block w-full px-3 py-1.5 text-left hover:bg-red-900/30"
							onclick={() => deleteView(v)}
						>
							Delete
						</button>
					</div>
				{/if}
			{/if}
		</div>
	{/each}
	<button
		class="ml-1 flex items-center gap-1 whitespace-nowrap rounded border border-dashed border-[var(--color-border)] px-2 py-1 text-[var(--color-muted)] hover:bg-white/5 hover:text-[var(--color-text)]"
		onclick={onSaveCurrent}
		title="Save current filter / sort / group as a new view"
	>
		<Icon name="plus" size={12} /> New view
	</button>
</div>

{#if renaming}
	{#await import('./SaveViewDialog.svelte') then mod}
		{@const Dialog = mod.default}
		<Dialog
			{entityType}
			mode="rename"
			existing={renaming}
			viewKind={renaming.kind}
			filter={renaming.filter}
			sortKey={renaming.sort_key}
			sortDir={renaming.sort_dir}
			groupByKey={renaming.group_by_key}
			columns={renaming.columns}
			cardProperties={renaming.card_properties}
			onCancel={() => (renaming = null)}
			onSaved={() => (renaming = null)}
		/>
	{/await}
{/if}

<style>
	.active {
		background-color: rgba(108, 182, 255, 0.15);
		color: var(--color-accent);
		border-color: var(--color-border);
	}
</style>
