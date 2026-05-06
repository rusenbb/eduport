<script lang="ts">
	import { filters } from '$lib/stores/filters';
</script>

{#if $filters.tags.length > 0 || $filters.level || $filters.country}
	<div class="flex flex-wrap items-center gap-2 border-b border-[var(--color-border)] px-4 py-2 text-xs">
		{#each $filters.tags as tag}
			<button
				class="rounded-full border border-[var(--color-accent)]/40 bg-[var(--color-accent)]/10 px-2 py-0.5 text-[var(--color-accent)] hover:bg-[var(--color-accent)]/20"
				onclick={() => filters.removeTag(tag)}
			>
				{tag} ✕
			</button>
		{/each}
		{#if $filters.level}
			<button
				class="rounded-full border border-[var(--color-warn)]/40 bg-[var(--color-warn)]/10 px-2 py-0.5 text-[var(--color-warn)]"
				onclick={() => filters.setLevel(undefined)}
			>
				level: {$filters.level} ✕
			</button>
		{/if}
		<button class="ml-auto text-[10px] uppercase text-[var(--color-muted)] hover:text-white" onclick={() => filters.clear()}>
			clear all
		</button>
	</div>
{/if}
