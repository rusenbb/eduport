<script lang="ts">
	let { rows = 6 }: { rows?: number } = $props();
</script>

<div class="flex flex-col" aria-hidden="true">
	{#each Array(rows) as _, i}
		<div
			class="grid grid-cols-[1fr_auto] items-center gap-3 border-b border-[var(--color-border)] px-4 py-3"
			style="opacity: {1 - i * 0.12};"
		>
			<div class="min-w-0 flex flex-col gap-1.5">
				<div class="h-3 w-2/3 rounded skeleton-bar"></div>
				<div class="h-2.5 w-1/3 rounded skeleton-bar"></div>
			</div>
			<div class="h-2.5 w-12 rounded skeleton-bar"></div>
		</div>
	{/each}
</div>

<style>
	.skeleton-bar {
		background: linear-gradient(
			90deg,
			rgba(255, 255, 255, 0.04) 0%,
			rgba(255, 255, 255, 0.10) 50%,
			rgba(255, 255, 255, 0.04) 100%
		);
		background-size: 200% 100%;
		animation: shimmer 1.4s ease-in-out infinite;
	}
	:global(:root[data-theme='light']) .skeleton-bar {
		background: linear-gradient(
			90deg,
			rgba(0, 0, 0, 0.04) 0%,
			rgba(0, 0, 0, 0.10) 50%,
			rgba(0, 0, 0, 0.04) 100%
		);
		background-size: 200% 100%;
		animation: shimmer 1.4s ease-in-out infinite;
	}
	@keyframes shimmer {
		0% { background-position: 200% 0; }
		100% { background-position: -200% 0; }
	}
</style>
