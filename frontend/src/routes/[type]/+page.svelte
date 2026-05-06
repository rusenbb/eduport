<script lang="ts">
	import { page } from '$app/state';
	import { listEntities } from '$lib/api/entities';
	import EntityList from '$lib/components/EntityList.svelte';
	import EntityForm from '$lib/components/EntityForm.svelte';
	import { filters } from '$lib/stores/filters';
	import { ENTITY_TYPES, type EntityListItem, type EntityType } from '$lib/types';
	import { getContext } from 'svelte';
	import type { Writable } from 'svelte/store';

	let items: EntityListItem[] = $state([]);
	let loading = $state(true);
	let error: string | null = $state(null);
	let creating = $state(false);

	const typeParam = $derived(page.params.type as string);
	const isValidType = $derived((ENTITY_TYPES as string[]).includes(typeParam));
	const type = $derived(typeParam as EntityType);

	const newAction = getContext<Writable<{ label: string; onClick: () => void } | null>>('eduport:newAction');

	async function load() {
		loading = true;
		error = null;
		try {
			items = await listEntities(type, $filters.tags);
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		if (isValidType) {
			newAction?.set({ label: `New ${type}`, onClick: () => (creating = true) });
			void load();
		}
		return () => newAction?.set(null);
	});
</script>

{#if !isValidType}
	<div class="p-8 text-center text-[var(--color-bad)]">Unknown entity type: {typeParam}</div>
{:else if loading}
	<div class="p-8 text-center text-[var(--color-muted)]">Loading…</div>
{:else if error}
	<div class="p-8 text-center text-[var(--color-bad)]">Error: {error}</div>
{:else}
	<EntityList {items} {type} />
{/if}

{#if creating}
	<EntityForm
		{type}
		onCancel={() => (creating = false)}
		onDone={() => {
			creating = false;
			void load();
		}}
	/>
{/if}
