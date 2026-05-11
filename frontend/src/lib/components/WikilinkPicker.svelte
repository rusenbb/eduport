<script lang="ts">
	import { listEntities } from '$lib/api/entities';
	import { asWikilink, targetOf } from '$lib/entities/meta';
	import type { EntityListItem, EntityType } from '$lib/types';
	import { onMount } from 'svelte';

	let {
		value = $bindable(''),
		type,
		placeholder = 'Select...',
		onChange
	}: {
		value: string;
		type: EntityType;
		placeholder?: string;
		onChange?: (value: string) => void;
	} = $props();

	let items: EntityListItem[] = $state([]);
	let query = $state('');
	let open = $state(false);

	const selectedTarget = $derived(targetOf(value));
	const selected = $derived(items.find((item) => item.file_id === selectedTarget));
	const filtered = $derived(
		items
			.filter((item) => {
				const q = query.trim().toLowerCase();
				return !q || item.name.toLowerCase().includes(q) || item.file_id.toLowerCase().includes(q);
			})
			.slice(0, 8)
	);

	onMount(async () => {
		items = await listEntities(type).catch(() => []);
	});

	function pick(item: EntityListItem) {
		setValue(asWikilink(item.file_id));
		query = '';
		open = false;
	}

	function setValue(next: string) {
		value = next;
		onChange?.(next);
	}

	function onKey(event: KeyboardEvent) {
		// ESC behaviour: if the dropdown is open, consume the key to
		// close just the dropdown. Otherwise let it bubble so the
		// surrounding dialog (EntityForm) can handle it. Without
		// stopPropagation, EntityForm's window-level keydown listener
		// would close the whole edit dialog every time the user hit
		// ESC to dismiss the suggestions popover.
		if (event.key === 'Escape' && open) {
			event.preventDefault();
			event.stopPropagation();
			open = false;
		}
	}

	function onBlur() {
		// Defer so a click on a dropdown row registers before the
		// dropdown closes. Same idiom as SelectCombobox.
		setTimeout(() => {
			open = false;
		}, 120);
	}
</script>

<div class="relative">
	<div class="flex gap-2">
		<input
			bind:value={query}
			onfocus={() => (open = true)}
			onkeydown={onKey}
			onblur={onBlur}
			placeholder={selected ? selected.name : placeholder}
			class="min-w-0 flex-1 rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-3 py-2 text-sm outline-none focus:border-[var(--color-accent)]"
		/>
		{#if value}
			<button
				type="button"
				class="rounded border border-[var(--color-border)] px-2 text-xs hover:bg-white/5"
				onclick={() => setValue('')}
			>
				Clear
			</button>
		{/if}
	</div>
	{#if selected && !query}
		<div class="mt-1 text-xs text-[var(--color-muted)]">{selected.file_id}</div>
	{/if}
	{#if open && filtered.length > 0}
		<div class="absolute z-20 mt-1 max-h-56 w-full overflow-auto rounded border border-[var(--color-border)] bg-[var(--color-panel)] shadow-xl">
			{#each filtered as item}
				<button
					type="button"
					class="block w-full border-b border-[var(--color-border)] px-3 py-2 text-left text-sm last:border-b-0 hover:bg-white/5"
					onmousedown={(event) => {
						event.preventDefault();
						pick(item);
					}}
				>
					<div>{item.name}</div>
					<div class="text-[10px] text-[var(--color-muted)]">{item.file_id}</div>
				</button>
			{/each}
		</div>
	{/if}
</div>
