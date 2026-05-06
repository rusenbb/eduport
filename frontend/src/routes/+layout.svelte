<script lang="ts">
	import '../app.css';
	import Sidebar from '$lib/components/Sidebar.svelte';
	import TopBar from '$lib/components/TopBar.svelte';
	import FilterChips from '$lib/components/FilterChips.svelte';
	import { setContext } from 'svelte';
	import { writable } from 'svelte/store';

	let { children } = $props();

	// Routes can register a contextual "+ New ..." action and a search opener.
	const newAction = writable<{ label: string; onClick: () => void } | null>(null);
	const searchOpener = writable<(() => void) | null>(null);
	setContext('eduport:newAction', newAction);
	setContext('eduport:searchOpener', searchOpener);

	function openSearch() {
		const fn = $searchOpener;
		if (fn) fn();
	}
</script>

<div class="flex h-screen w-screen overflow-hidden">
	<Sidebar />
	<div class="flex flex-1 flex-col overflow-hidden">
		<TopBar onSearch={openSearch} newAction={$newAction ?? undefined} />
		<FilterChips />
		<div class="flex-1 overflow-auto">
			{@render children()}
		</div>
	</div>
</div>
