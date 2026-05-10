<script lang="ts">
	import type { PropertyType } from '$lib/types/schema';

	let {
		type,
		size = 12,
		class: className = ''
	}: { type: PropertyType; size?: number; class?: string } = $props();
</script>

<svg
	xmlns="http://www.w3.org/2000/svg"
	width={size}
	height={size}
	viewBox="0 0 24 24"
	fill="none"
	stroke="currentColor"
	stroke-width="2"
	stroke-linecap="round"
	stroke-linejoin="round"
	class={className}
	aria-hidden="true"
>
	{#if type === 'text'}
		<polyline points="4 7 4 4 20 4 20 7" />
		<line x1="9" x2="15" y1="20" y2="20" />
		<line x1="12" x2="12" y1="4" y2="20" />
	{:else if type === 'number'}
		<line x1="4" x2="20" y1="9" y2="9" />
		<line x1="4" x2="20" y1="15" y2="15" />
		<line x1="10" x2="8" y1="3" y2="21" />
		<line x1="16" x2="14" y1="3" y2="21" />
	{:else if type === 'date'}
		<rect width="18" height="18" x="3" y="4" rx="2" ry="2" />
		<line x1="16" x2="16" y1="2" y2="6" />
		<line x1="8" x2="8" y1="2" y2="6" />
		<line x1="3" x2="21" y1="10" y2="10" />
	{:else if type === 'checkbox'}
		<rect width="18" height="18" x="3" y="3" rx="2" ry="2" />
		<polyline points="9 12 11 14 15 10" />
	{:else if type === 'single-select'}
		<circle cx="12" cy="12" r="9" />
		<circle cx="12" cy="12" r="3" fill="currentColor" />
	{:else if type === 'multi-select'}
		<rect width="6" height="6" x="3" y="4" rx="1" />
		<rect width="6" height="6" x="3" y="14" rx="1" />
		<line x1="12" x2="20" y1="7" y2="7" />
		<line x1="12" x2="20" y1="17" y2="17" />
	{:else if type === 'url'}
		<path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71" />
		<path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71" />
	{:else if type === 'relation'}
		<path d="M9 11H5a2 2 0 0 0-2 2v6a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2v-4" />
		<path d="M21 13V5a2 2 0 0 0-2-2h-8" />
		<path d="m15 9 6-6" />
	{/if}
</svg>
