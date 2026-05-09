<script lang="ts">
	/**
	 * Per-kanban card-properties picker. Drives the active saved view's
	 * `card_properties` field — which keys appear under the title on each
	 * card. When no view is active, this is a no-op (the picker is hidden
	 * by EntityWorkspace).
	 */
	import { viewsStore } from '$lib/stores/views';
	import type { EntityType } from '$lib/types';
	import type { Property } from '$lib/types/schema';
	import type { View } from '$lib/types/view';
	import Icon from '../Icon.svelte';
	import PropertyTypeIcon from './PropertyTypeIcon.svelte';

	let {
		entityType,
		properties,
		activeView
	}: {
		entityType: EntityType;
		properties: Property[];
		activeView: View;
	} = $props();

	let open = $state(false);
	const selected = $derived(new Set(activeView.card_properties ?? []));

	async function toggle(key: string) {
		const next = new Set(selected);
		if (next.has(key)) next.delete(key);
		else next.add(key);
		const orderedKeys = properties.filter((p) => next.has(p.key)).map((p) => p.key);
		await viewsStore.update(entityType, activeView.id, {
			name: activeView.name,
			kind: activeView.kind,
			filter: activeView.filter,
			sort_key: activeView.sort_key,
			sort_dir: activeView.sort_dir,
			group_by_key: activeView.group_by_key,
			columns: activeView.columns,
			card_properties: orderedKeys.length > 0 ? orderedKeys : null
		});
	}
</script>

<div class="relative">
	<button
		class="rounded border border-[var(--color-border)] bg-white/5 px-2 py-1 text-xs hover:bg-white/10"
		onclick={() => (open = !open)}
	>
		Card props ({selected.size})
	</button>
	{#if open}
		<div class="absolute right-0 top-full z-30 mt-1 w-56 overflow-hidden rounded border border-[var(--color-border)] bg-[var(--color-panel)] text-xs shadow-xl">
			<div class="max-h-72 overflow-auto">
				{#if properties.length === 0}
					<div class="px-3 py-2 text-[var(--color-muted)]">No custom properties yet.</div>
				{:else}
					{#each properties as prop}
						<button
							class="flex w-full items-center gap-2 border-b border-[var(--color-border)] px-3 py-1.5 text-left last:border-b-0 hover:bg-white/5"
							onclick={() => toggle(prop.key)}
						>
							<input type="checkbox" checked={selected.has(prop.key)} class="pointer-events-none" />
							<PropertyTypeIcon type={prop.type} class="text-[var(--color-muted)]" />
							<span class="truncate">{prop.name}</span>
						</button>
					{/each}
				{/if}
			</div>
			<div class="border-t border-[var(--color-border)] px-3 py-1.5 text-right">
				<button class="text-[var(--color-muted)]" onclick={() => (open = false)}>
					<Icon name="x" size={12} />
				</button>
			</div>
		</div>
	{/if}
</div>
