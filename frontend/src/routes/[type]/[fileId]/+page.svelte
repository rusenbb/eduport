<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { deleteEntity, getEntity } from '$lib/api/entities';
	import DetailPanel from '$lib/components/DetailPanel.svelte';
	import { ENTITY_TYPES, type EntityDetail, type EntityType } from '$lib/types';

	let detail: EntityDetail | null = $state(null);
	let loading = $state(true);
	let error: string | null = $state(null);

	const type = $derived(page.params.type as EntityType);
	const fileId = $derived(page.params.fileId as string);
	const isValid = $derived((ENTITY_TYPES as string[]).includes(type));

	async function load() {
		loading = true;
		error = null;
		try {
			detail = await getEntity(type, fileId);
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		if (isValid && fileId) {
			void load();
		}
	});

	async function handleDelete() {
		if (!detail) return;
		if (!confirm(`Move "${detail.entity.name}" to trash?`)) return;
		try {
			await deleteEntity(type, fileId);
			goto(`/${type}`);
		} catch (e) {
			alert(`Delete failed: ${e instanceof Error ? e.message : String(e)}`);
		}
	}
</script>

{#if !isValid}
	<div class="p-8 text-center text-[var(--color-bad)]">Unknown entity type: {type}</div>
{:else if loading}
	<div class="p-8 text-center text-[var(--color-muted)]">Loading…</div>
{:else if error}
	<div class="p-8 text-center text-[var(--color-bad)]">Error: {error}</div>
{:else if detail}
	<DetailPanel {detail} onDelete={handleDelete} />
{/if}
