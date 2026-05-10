<script lang="ts">
	import '../app.css';
	import Sidebar from '$lib/components/Sidebar.svelte';
	import TopBar from '$lib/components/TopBar.svelte';
	import FilterChips from '$lib/components/FilterChips.svelte';
	import CommandPalette from '$lib/components/CommandPalette.svelte';
	import EntityForm from '$lib/components/EntityForm.svelte';
	import FirstRunPrompt from '$lib/components/FirstRunPrompt.svelte';
	import { parseEml } from '$lib/api/eml';
	import { settings } from '$lib/stores/settings';
	import { status } from '$lib/stores/status';
	import { isTauri, readFileBytes } from '$lib/tauri';
	import { onMount, setContext } from 'svelte';
	import { writable } from 'svelte/store';

	let { data, children } = $props();

	let paletteOpen = $state(false);
	let droppedEmail:
		| {
				frontmatter: Record<string, unknown>;
				body: string;
		  }
		| null = $state(null);

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
		let unlistenDrop: (() => void) | null = null;
		if (isTauri()) {
			void import('@tauri-apps/api/window').then(async ({ getCurrentWindow }) => {
				unlistenDrop = await getCurrentWindow().onDragDropEvent(async (event) => {
					if (event.payload.type !== 'drop') return;
					const emlPath = event.payload.paths.find((path) => path.toLowerCase().endsWith('.eml'));
					if (!emlPath) return;
					try {
						const bytes = await readFileBytes(emlPath);
						const copy = new Uint8Array(bytes);
						const parsed = await parseEml(new Blob([copy.buffer as ArrayBuffer]), emlPath.split('/').pop() ?? 'message.eml');
						droppedEmail = {
							frontmatter: {
								tags: ['eduport-type/email', 'imported'],
								name: parsed.subject || 'Imported email',
								direction: parsed.direction,
								date: parsed.date ?? new Date().toISOString().slice(0, 10),
								subject: parsed.subject,
								from: parsed.from,
								to: parsed.to,
								cc: parsed.cc,
								bcc: parsed.bcc
							},
							body: parsed.body
						};
					} catch (e) {
						alert(`Email import failed: ${e instanceof Error ? e.message : String(e)}`);
					}
				});
			});
		}
		return () => {
			window.removeEventListener('keydown', onKey);
			unlistenDrop?.();
		};
	});

	$effect(() => {
		const theme = $settings?.theme ?? 'system';
		document.documentElement.dataset.theme = theme;
	});
</script>

{#if data.hasSettings === false}
	<FirstRunPrompt />
{:else if !$status.coreUp && $status.lastChecked > 0}
	<div class="flex h-screen w-screen items-center justify-center p-6 text-center">
		<div>
			<h1 class="text-xl font-semibold text-[var(--color-bad)]">Backend offline</h1>
			<p class="mt-2 text-sm text-[var(--color-muted)]">
				Eduport's core can't open the vault. Check that the data folder in
				<a href="/settings" class="underline">Settings</a> exists and is readable.
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
	{#if droppedEmail}
		<EntityForm
			type="email"
			initial={droppedEmail}
			onCancel={() => (droppedEmail = null)}
			onDone={() => (droppedEmail = null)}
		/>
	{/if}
{/if}
