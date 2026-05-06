<script lang="ts">
	import { page } from '$app/state';
	import EntityWorkspace from '$lib/components/EntityWorkspace.svelte';
	import { ENTITY_TYPES, type EntityType } from '$lib/types';

	const type = $derived(page.params.type as EntityType);
	const fileId = $derived(page.params.fileId as string);
	const isValid = $derived((ENTITY_TYPES as string[]).includes(type));
</script>

{#if !isValid}
	<div class="p-8 text-center text-[var(--color-bad)]">Unknown entity type: {type}</div>
{:else}
	<EntityWorkspace {type} {fileId} />
{/if}
