<script lang="ts">
	import { listEntities } from '$lib/api/entities';
	import type { EntityListItem, EntityType } from '$lib/types';
	import { ENTITY_TYPES } from '$lib/types';
	import { COLOR_CLASSES, type Property, type SelectOption } from '$lib/types/schema';
	import { onMount } from 'svelte';

	let {
		prop,
		value = $bindable(),
		onChange
	}: {
		prop: Property;
		value: unknown;
		onChange?: (next: unknown) => void;
	} = $props();

	function set(next: unknown) {
		value = next;
		onChange?.(next);
	}

	function targetOf(link: string | null | undefined): string | null {
		if (!link) return null;
		const m = /^\[\[(.+)\]\]$/.exec(link.trim());
		return m ? m[1] : null;
	}

	// ---- relation autocomplete -------------------------------------------------
	let relationItems: EntityListItem[] = $state([]);
	let relationQuery = $state('');
	let relationOpen = $state(false);

	async function loadRelationCandidates() {
		if (prop.type !== 'relation') return;
		const types: EntityType[] =
			prop.target_types && prop.target_types.length > 0 ? prop.target_types : ENTITY_TYPES;
		const lists = await Promise.all(types.map((t) => listEntities(t).catch(() => [])));
		relationItems = lists.flat();
	}

	onMount(() => {
		void loadRelationCandidates();
	});

	const filteredRelationItems = $derived(
		(() => {
			const q = relationQuery.trim().toLowerCase();
			if (!q) return relationItems.slice(0, 10);
			return relationItems
				.filter(
					(item) =>
						item.name.toLowerCase().includes(q) || item.file_id.toLowerCase().includes(q)
				)
				.slice(0, 10);
		})()
	);

	function pickRelation(item: EntityListItem) {
		set(`[[${item.file_id}]]`);
		relationQuery = '';
		relationOpen = false;
	}

	const currentRelationLabel = $derived(
		(() => {
			if (prop.type !== 'relation' || typeof value !== 'string') return null;
			const target = targetOf(value);
			if (!target) return null;
			const item = relationItems.find((i) => i.file_id === target);
			return item ? `${item.name}` : target;
		})()
	);

	// ---- multi-select toggle ---------------------------------------------------
	function toggleMulti(opt: SelectOption) {
		const arr = Array.isArray(value) ? [...(value as string[])] : [];
		const idx = arr.indexOf(opt.value);
		if (idx >= 0) arr.splice(idx, 1);
		else arr.push(opt.value);
		set(arr);
	}

	const numberInput = $derived(
		typeof value === 'number' ? String(value) : value == null ? '' : String(value)
	);
</script>

{#if prop.type === 'text'}
	<input
		value={typeof value === 'string' ? value : ''}
		oninput={(e) => set((e.currentTarget as HTMLInputElement).value)}
		class="w-full rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-3 py-2 text-sm outline-none focus:border-[var(--color-accent)]"
	/>
{:else if prop.type === 'number'}
	<div class="flex items-center gap-2">
		<input
			type="number"
			value={numberInput}
			oninput={(e) => {
				const v = (e.currentTarget as HTMLInputElement).value;
				set(v === '' ? null : Number(v));
			}}
			class="w-32 rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-3 py-2 text-sm outline-none focus:border-[var(--color-accent)]"
		/>
		{#if prop.unit}
			<span class="text-xs text-[var(--color-muted)]">{prop.unit}</span>
		{/if}
	</div>
{:else if prop.type === 'date'}
	<input
		type="date"
		value={typeof value === 'string' ? value : ''}
		oninput={(e) => set((e.currentTarget as HTMLInputElement).value || null)}
		class="rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-3 py-2 text-sm outline-none focus:border-[var(--color-accent)]"
	/>
{:else if prop.type === 'checkbox'}
	<label class="inline-flex items-center gap-2 text-sm">
		<input
			type="checkbox"
			checked={value === true}
			onchange={(e) => set((e.currentTarget as HTMLInputElement).checked)}
		/>
		<span>{value === true ? 'Yes' : 'No'}</span>
	</label>
{:else if prop.type === 'url'}
	<input
		type="url"
		value={typeof value === 'string' ? value : ''}
		placeholder="https://..."
		oninput={(e) => set((e.currentTarget as HTMLInputElement).value || null)}
		class="w-full rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-3 py-2 text-sm outline-none focus:border-[var(--color-accent)]"
	/>
{:else if prop.type === 'single-select'}
	<div class="flex flex-wrap gap-1.5">
		{#each prop.options as opt}
			{@const c = COLOR_CLASSES[opt.color]}
			{@const active = value === opt.value}
			<button
				type="button"
				class="rounded border px-2 py-0.5 text-xs {c.bg} {c.text} {active ? c.border : 'border-transparent'} hover:{c.border}"
				onclick={() => set(active ? null : opt.value)}
			>
				{opt.label}
			</button>
		{/each}
		{#if value && !prop.options.find((o) => o.value === value)}
			<span class="rounded border border-[var(--color-border)] bg-white/5 px-2 py-0.5 text-xs">
				{String(value)} <span class="text-[10px] text-[var(--color-muted)]">(off-list)</span>
			</span>
		{/if}
	</div>
{:else if prop.type === 'multi-select'}
	<div class="flex flex-wrap gap-1.5">
		{#each prop.options as opt}
			{@const c = COLOR_CLASSES[opt.color]}
			{@const active = Array.isArray(value) && (value as string[]).includes(opt.value)}
			<button
				type="button"
				class="rounded border px-2 py-0.5 text-xs {c.bg} {c.text} {active ? c.border : 'border-transparent'} hover:{c.border}"
				onclick={() => toggleMulti(opt)}
			>
				{opt.label}
			</button>
		{/each}
	</div>
{:else if prop.type === 'relation'}
	<div class="relative">
		<div class="flex gap-2">
			<input
				value={relationQuery}
				oninput={(e) => (relationQuery = (e.currentTarget as HTMLInputElement).value)}
				onfocus={() => (relationOpen = true)}
				placeholder={currentRelationLabel ?? 'Search...'}
				class="min-w-0 flex-1 rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-3 py-2 text-sm outline-none focus:border-[var(--color-accent)]"
			/>
			{#if value}
				<button
					type="button"
					class="rounded border border-[var(--color-border)] px-2 text-xs hover:bg-white/5"
					onclick={() => set(null)}
				>
					Clear
				</button>
			{/if}
		</div>
		{#if currentRelationLabel && !relationQuery}
			<div class="mt-1 text-xs text-[var(--color-muted)]">{targetOf(String(value))}</div>
		{/if}
		{#if relationOpen && filteredRelationItems.length > 0}
			<div class="absolute z-20 mt-1 max-h-56 w-full overflow-auto rounded border border-[var(--color-border)] bg-[var(--color-panel)] shadow-xl">
				{#each filteredRelationItems as item}
					<button
						type="button"
						class="block w-full border-b border-[var(--color-border)] px-3 py-2 text-left text-sm last:border-b-0 hover:bg-white/5"
						onmousedown={(event) => {
							event.preventDefault();
							pickRelation(item);
						}}
					>
						<div>{item.name}</div>
						<div class="text-[10px] text-[var(--color-muted)]">{item.file_id} · {item.type}</div>
					</button>
				{/each}
			</div>
		{/if}
	</div>
{/if}
