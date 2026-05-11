<script lang="ts">
	import { toasts } from '$lib/stores/toasts';
	import { fly } from 'svelte/transition';
	import Icon from './Icon.svelte';
</script>

<!-- Rendered once at the root layout. Pointer-events on the container
	 are 'none' so the toast region doesn't block clicks underneath when
	 empty; individual toasts re-enable pointer events for their own
	 dismiss button. -->
<div
	class="pointer-events-none fixed bottom-4 right-4 z-50 flex max-w-sm flex-col gap-2"
	aria-live="polite"
	aria-atomic="false"
>
	{#each $toasts as t (t.id)}
		<div
			transition:fly={{ y: 20, duration: 180 }}
			class="pointer-events-auto flex items-start gap-2 rounded border px-3 py-2 text-xs shadow-xl"
			class:success={t.kind === 'success'}
			class:error={t.kind === 'error'}
			class:info={t.kind === 'info'}
			role="status"
		>
			<div class="min-w-0 flex-1">
				<div class="font-medium">{t.message}</div>
				{#if t.detail}
					<div class="mt-0.5 text-[10px] opacity-70">{t.detail}</div>
				{/if}
			</div>
			<button
				class="-mr-1 flex-shrink-0 rounded p-0.5 opacity-70 hover:opacity-100"
				onclick={() => toasts.dismiss(t.id)}
				aria-label="Dismiss"
			>
				<Icon name="x" size={12} />
			</button>
		</div>
	{/each}
</div>

<style>
	.success {
		color: var(--color-good);
		background-color: rgba(98, 196, 84, 0.15);
		border-color: rgba(98, 196, 84, 0.4);
	}
	.error {
		color: var(--color-bad);
		background-color: rgba(224, 137, 137, 0.15);
		border-color: rgba(224, 137, 137, 0.4);
	}
	.info {
		color: var(--color-text);
		background-color: var(--color-panel);
		border-color: var(--color-border);
	}
</style>
