<script lang="ts" module>
	// Inline SVG icons (Lucide, ISC-licensed). One file keeps the set discoverable
	// and lets the union type below catch typos at compile time.
	export type IconName =
		| 'dashboard'
		| 'deadlines'
		| 'status'
		| 'program'
		| 'application'
		| 'person'
		| 'university'
		| 'lab'
		| 'document'
		| 'email'
		| 'note'
		| 'trash'
		| 'settings'
		| 'link'
		| 'warning'
		| 'plus'
		| 'x'
		| 'check'
		| 'sliders';
</script>

<script lang="ts">
	let {
		name,
		size = 16,
		class: className = ''
	}: { name: IconName; size?: number; class?: string } = $props();
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
	{#if name === 'dashboard'}
		<rect width="7" height="9" x="3" y="3" rx="1" />
		<rect width="7" height="5" x="14" y="3" rx="1" />
		<rect width="7" height="9" x="14" y="12" rx="1" />
		<rect width="7" height="5" x="3" y="16" rx="1" />
	{:else if name === 'deadlines'}
		<path d="M21 7.5V6a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h3.5" />
		<path d="M16 2v4" /><path d="M8 2v4" /><path d="M3 10h18" />
		<circle cx="16" cy="16" r="6" /><path d="M16 14v2l1 1" />
	{:else if name === 'status'}
		<path d="M22 12h-2.48a2 2 0 0 0-1.93 1.46l-2.35 8.36a.5.5 0 0 1-.96 0L9.68 3.18a.5.5 0 0 0-.96 0l-2.35 8.36A2 2 0 0 1 4.44 13H2" />
	{:else if name === 'program'}
		<path d="M21.42 10.922a1 1 0 0 0-.019-1.838L12.83 5.18a2 2 0 0 0-1.66 0L2.6 9.08a1 1 0 0 0 0 1.832l8.57 3.908a2 2 0 0 0 1.66 0z" />
		<path d="M22 10v6" />
		<path d="M6 12.5V16a6 3 0 0 0 12 0v-3.5" />
	{:else if name === 'application'}
		<rect width="8" height="4" x="8" y="2" rx="1" ry="1" />
		<path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2" />
		<path d="M12 11h4" /><path d="M12 16h4" /><path d="M8 11h.01" /><path d="M8 16h.01" />
	{:else if name === 'person'}
		<path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2" />
		<circle cx="9" cy="7" r="4" />
		<path d="M22 21v-2a4 4 0 0 0-3-3.87" />
		<path d="M16 3.13a4 4 0 0 1 0 7.75" />
	{:else if name === 'university'}
		<path d="M14 22v-4a2 2 0 1 0-4 0v4" />
		<path d="m18 10 4 2v8a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2v-8l4-2" />
		<path d="M18 5v17" /><path d="m4 6 7.106-3.553a2 2 0 0 1 1.788 0L20 6" />
		<path d="M6 5v17" /><circle cx="12" cy="9" r="2" />
	{:else if name === 'lab'}
		<path d="M10 2v7.31" /><path d="M14 9.3V1.99" />
		<path d="M8.5 2h7" />
		<path d="M14 9.3a6.5 6.5 0 1 1-4 0" />
		<path d="M5.52 16h12.96" />
	{:else if name === 'document'}
		<path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" />
		<path d="M14 2v4a2 2 0 0 0 2 2h4" />
		<path d="M10 9H8" /><path d="M16 13H8" /><path d="M16 17H8" />
	{:else if name === 'email'}
		<rect width="20" height="16" x="2" y="4" rx="2" />
		<path d="m22 7-8.97 5.7a1.94 1.94 0 0 1-2.06 0L2 7" />
	{:else if name === 'note'}
		<path d="M2 6a2 2 0 0 1 2-2h12l4 4v12a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2z" />
		<path d="M2 10h20" /><path d="M6 14h2" /><path d="M6 18h2" />
		<path d="M12 14h4" /><path d="M12 18h4" />
	{:else if name === 'trash'}
		<path d="M3 6h18" />
		<path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6" />
		<path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
		<line x1="10" x2="10" y1="11" y2="17" />
		<line x1="14" x2="14" y1="11" y2="17" />
	{:else if name === 'settings'}
		<path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" />
		<circle cx="12" cy="12" r="3" />
	{:else if name === 'link'}
		<path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71" />
		<path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71" />
	{:else if name === 'warning'}
		<path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z" />
		<line x1="12" x2="12" y1="9" y2="13" />
		<line x1="12" x2="12.01" y1="17" y2="17" />
	{:else if name === 'plus'}
		<path d="M12 5v14" /><path d="M5 12h14" />
	{:else if name === 'x'}
		<path d="M18 6 6 18" /><path d="m6 6 12 12" />
	{:else if name === 'check'}
		<path d="M20 6 9 17l-5-5" />
	{:else if name === 'sliders'}
		<line x1="4" x2="4" y1="21" y2="14" />
		<line x1="4" x2="4" y1="10" y2="3" />
		<line x1="12" x2="12" y1="21" y2="12" />
		<line x1="12" x2="12" y1="8" y2="3" />
		<line x1="20" x2="20" y1="21" y2="16" />
		<line x1="20" x2="20" y1="12" y2="3" />
		<line x1="2" x2="6" y1="14" y2="14" />
		<line x1="10" x2="14" y1="8" y2="8" />
		<line x1="18" x2="22" y1="16" y2="16" />
	{/if}
</svg>
