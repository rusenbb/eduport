<script lang="ts">
	import { page } from '$app/state';
	import { listEntities } from '$lib/api/entities';
	import EntityList from '$lib/components/EntityList.svelte';
	import { filters } from '$lib/stores/filters';
	import { ENTITY_TYPES, type EntityListItem, type EntityType } from '$lib/types';
	import { onMount } from 'svelte';

	let items: EntityListItem[] = $state([]);
	let loading = $state(true);
	let error: string | null = $state(null);

	const typeParam = $derived(page.params.type as string);
	const isValidType = $derived((ENTITY_TYPES as string[]).includes(typeParam));
	const type = $derived(typeParam as EntityType);

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
		// Re-fetch on type change or filters change
		if (isValidType) {
			void load();
		}
	});

	onMount(() => {
		// Mark `void` so the lint doesn't complain about an unused promise
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
