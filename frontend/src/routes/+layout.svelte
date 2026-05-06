<script lang="ts">
	import '../app.css';
	import Sidebar from '$lib/components/Sidebar.svelte';
	import TopBar from '$lib/components/TopBar.svelte';
	import FilterChips from '$lib/components/FilterChips.svelte';
	import CommandPalette from '$lib/components/CommandPalette.svelte';
	import FirstRunPrompt from '$lib/components/FirstRunPrompt.svelte';
	import { status } from '$lib/stores/status';
	import { onMount, setContext } from 'svelte';
	import { writable } from 'svelte/store';

	let { data, children } = $props();

	let paletteOpen = $state(false);

	const newAction = writable<{ label: string; onClick: () => void } | null>(null);
	setContext('eduport:newAction', newAction);

	function openPalette() {
		paletteOpen = true;
	}

	function onKey(event: KeyboardEvent) {
		if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
			event.preventDefault();
			openPalette();
		}
	}

	onMount(() => {
		window.addEventListener('keydown', onKey);
		return () => window.removeEventListener('keydown', onKey);
	});
</script>

{#if data.hasSettings === false && $status.sidecarUp}
	<FirstRunPrompt />
{:else if !$status.sidecarUp && $status.lastChecked > 0}
	<div class="flex h-screen w-screen items-center justify-center p-6 text-center">
		<div>
			<h1 class="text-xl font-semibold text-[var(--color-bad)]">Backend offline</h1>
			<p class="mt-2 text-sm text-[var(--color-muted)]">
				Eduport can't reach its sidecar. Make sure <code class="rounded bg-white/5 px-1">eduport-sidecar</code>
				is running and reachable.
			</p>
			<button class="mt-4 rounded border border-[var(--color-border)] bg-white/5 px-3 py-1.5 text-xs hover:bg-white/10" onclick={() => status.check()}>
				Retry
			</button>
		</div>
	</div>
{:else}
	<div class="flex h-screen w-screen overflow-hidden">
		<Sidebar />
		<div class="flex flex-1 flex-col overflow-hidden">
			<TopBar onSearch={openPalette} newAction={$newAction ?? undefined} />
			<FilterChips />
			<div class="flex-1 overflow-auto">
				{@render children()}
			</div>
		</div>
	</div>

	<CommandPalette bind:open={paletteOpen} />
{/if}
