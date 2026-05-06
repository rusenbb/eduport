<script lang="ts">
	import { page } from '$app/state';
	import EntityWorkspace from '$lib/components/EntityWorkspace.svelte';
	import { ENTITY_TYPES, type EntityType } from '$lib/types';

	const typeParam = $derived(page.params.type as string);
	const isValidType = $derived((ENTITY_TYPES as string[]).includes(typeParam));
	const type = $derived(typeParam as EntityType);
</script>

{#if !isValidType}
	<div class="p-8 text-center text-[var(--color-bad)]">Unknown entity type: {typeParam}</div>
{:else}
	<EntityWorkspace {type} />
{/if}
