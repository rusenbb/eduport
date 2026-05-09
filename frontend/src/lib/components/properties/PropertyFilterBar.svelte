<script lang="ts">
	import Icon from '../Icon.svelte';
	import {
		COLOR_CLASSES,
		DEFAULT_PROPERTY_FILTERS,
		type Property,
		type PropertyFilters
	} from '$lib/types/schema';

	let {
		properties,
		filters = $bindable(DEFAULT_PROPERTY_FILTERS),
		onChange
	}: {
		properties: Property[];
		filters: PropertyFilters;
		onChange?: (next: PropertyFilters) => void;
	} = $props();

	let addOpen = $state(false);

	function update(next: PropertyFilters) {
		filters = next;
		onChange?.(next);
	}

	function activeKeys(): Set<string> {
		return new Set([
			...Object.keys(filters.text),
			...Object.keys(filters.num),
			...Object.keys(filters.date)
		]);
	}

	function isFilterableType(p: Property): boolean {
		// All except checkbox-without-tri-state — but for simplicity we allow
		// all types; checkbox is handled below via "is true" / "is false".
		return p.type !== 'relation' || true;
	}

	const sortableProperties = $derived(properties.filter((p) => p.type !== 'relation'));

	function startFilter(prop: Property) {
		const key = prop.key;
		const next: PropertyFilters = {
			text: { ...filters.text },
			num: { ...filters.num },
			date: { ...filters.date },
			sort: filters.sort,
			sortDir: filters.sortDir
		};
		switch (prop.type) {
			case 'text':
			case 'url':
			case 'single-select':
			case 'multi-select':
			case 'relation':
				next.text[key] = '';
				break;
			case 'number':
				next.num[key] = [null, null];
				break;
			case 'date':
				next.date[key] = [null, null];
				break;
			case 'checkbox':
				next.text[key] = ''; // 'true' / 'false'
				break;
		}
		update(next);
		addOpen = false;
	}

	function removeFilter(key: string) {
		const next: PropertyFilters = {
			text: { ...filters.text },
			num: { ...filters.num },
			date: { ...filters.date },
			sort: filters.sort,
			sortDir: filters.sortDir
		};
		delete next.text[key];
		delete next.num[key];
		delete next.date[key];
		update(next);
	}

	function setText(key: string, value: string) {
		update({ ...filters, text: { ...filters.text, [key]: value } });
	}

	function setNum(key: string, lo: number | null, hi: number | null) {
		update({ ...filters, num: { ...filters.num, [key]: [lo, hi] } });
	}

	function setDate(key: string, lo: string | null, hi: string | null) {
		update({ ...filters, date: { ...filters.date, [key]: [lo, hi] } });
	}

	function setSort(key: string | undefined, dir: 'asc' | 'desc' = 'asc') {
		update({ ...filters, sort: key || undefined, sortDir: key ? dir : undefined });
	}

	function clearAll() {
		update({ text: {}, num: {}, date: {} });
	}

	const inactive = $derived(properties.filter((p) => !activeKeys().has(p.key)));
</script>

<div class="flex flex-wrap items-center gap-2 border-b border-[var(--color-border)] px-4 py-2 text-xs">
	<!-- Active filter chips -->
	{#each Object.entries(filters.text) as [key, value]}
		{@const prop = properties.find((p) => p.key === key)}
		{#if prop}
			<div class="flex items-center gap-1 rounded border border-[var(--color-border)] bg-white/5 px-2 py-1">
				<span class="font-medium">{prop.name}</span>
				{#if prop.type === 'single-select' || prop.type === 'multi-select'}
					<select
						value={value}
						onchange={(e) => setText(key, (e.currentTarget as HTMLSelectElement).value)}
						class="rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-1 py-0.5 text-[10px] text-[var(--color-text)]"
					>
						<option value="">(any)</option>
						{#each prop.options as opt}
							<option value={opt.value}>{opt.label}</option>
						{/each}
					</select>
				{:else if prop.type === 'checkbox'}
					<select
						value={value}
						onchange={(e) => setText(key, (e.currentTarget as HTMLSelectElement).value)}
						class="rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-1 py-0.5 text-[10px]"
					>
						<option value="">(any)</option>
						<option value="true">true</option>
						<option value="false">false</option>
					</select>
				{:else}
					<input
						value={value}
						oninput={(e) => setText(key, (e.currentTarget as HTMLInputElement).value)}
						placeholder="contains..."
						class="w-24 rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-1 py-0.5 text-[10px]"
					/>
				{/if}
				<button class="text-[var(--color-muted)] hover:text-[var(--color-text)]" onclick={() => removeFilter(key)} aria-label="Remove">
					<Icon name="x" size={12} />
				</button>
			</div>
		{/if}
	{/each}

	{#each Object.entries(filters.num) as [key, [lo, hi]]}
		{@const prop = properties.find((p) => p.key === key)}
		{#if prop}
			<div class="flex items-center gap-1 rounded border border-[var(--color-border)] bg-white/5 px-2 py-1">
				<span class="font-medium">{prop.name}</span>
				<input
					type="number"
					value={lo ?? ''}
					oninput={(e) =>
						setNum(key, (e.currentTarget as HTMLInputElement).value === '' ? null : Number((e.currentTarget as HTMLInputElement).value), hi)}
					placeholder="min"
					class="w-16 rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-1 py-0.5 text-[10px]"
				/>
				<span class="text-[10px] text-[var(--color-muted)]">to</span>
				<input
					type="number"
					value={hi ?? ''}
					oninput={(e) =>
						setNum(key, lo, (e.currentTarget as HTMLInputElement).value === '' ? null : Number((e.currentTarget as HTMLInputElement).value))}
					placeholder="max"
					class="w-16 rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-1 py-0.5 text-[10px]"
				/>
				<button class="text-[var(--color-muted)] hover:text-[var(--color-text)]" onclick={() => removeFilter(key)} aria-label="Remove">
					<Icon name="x" size={12} />
				</button>
			</div>
		{/if}
	{/each}

	{#each Object.entries(filters.date) as [key, [lo, hi]]}
		{@const prop = properties.find((p) => p.key === key)}
		{#if prop}
			<div class="flex items-center gap-1 rounded border border-[var(--color-border)] bg-white/5 px-2 py-1">
				<span class="font-medium">{prop.name}</span>
				<input
					type="date"
					value={lo ?? ''}
					oninput={(e) =>
						setDate(key, (e.currentTarget as HTMLInputElement).value || null, hi)}
					class="rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-1 py-0.5 text-[10px]"
				/>
				<span class="text-[10px] text-[var(--color-muted)]">to</span>
				<input
					type="date"
					value={hi ?? ''}
					oninput={(e) =>
						setDate(key, lo, (e.currentTarget as HTMLInputElement).value || null)}
					class="rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-1 py-0.5 text-[10px]"
				/>
				<button class="text-[var(--color-muted)] hover:text-[var(--color-text)]" onclick={() => removeFilter(key)} aria-label="Remove">
					<Icon name="x" size={12} />
				</button>
			</div>
		{/if}
	{/each}

	<!-- Add filter dropdown -->
	{#if inactive.length > 0}
		<div class="relative">
			<button
				class="rounded border border-[var(--color-border)] px-2 py-1 hover:bg-white/5"
				onclick={() => (addOpen = !addOpen)}
			>
				<Icon name="plus" size={12} /> Filter
			</button>
			{#if addOpen}
				<div class="absolute left-0 top-full z-30 mt-1 w-48 overflow-hidden rounded border border-[var(--color-border)] bg-[var(--color-panel)] shadow-xl">
					{#each inactive as prop}
						<button
							class="block w-full border-b border-[var(--color-border)] px-3 py-1.5 text-left text-xs last:border-b-0 hover:bg-white/5"
							onclick={() => startFilter(prop)}
						>
							{prop.name}
							<span class="text-[10px] text-[var(--color-muted)]">· {prop.type}</span>
						</button>
					{/each}
				</div>
			{/if}
		</div>
	{/if}

	<!-- Sort -->
	{#if sortableProperties.length > 0}
		<div class="ml-auto flex items-center gap-1">
			<span class="text-[var(--color-muted)]">Sort</span>
			<select
				value={filters.sort ?? ''}
				onchange={(e) => setSort((e.currentTarget as HTMLSelectElement).value || undefined, filters.sortDir ?? 'asc')}
				class="rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-1 py-0.5 text-[10px]"
			>
				<option value="">(name)</option>
				{#each sortableProperties as prop}
					<option value={prop.key}>{prop.name}</option>
				{/each}
			</select>
			{#if filters.sort}
				<select
					value={filters.sortDir ?? 'asc'}
					onchange={(e) => setSort(filters.sort, (e.currentTarget as HTMLSelectElement).value as 'asc' | 'desc')}
					class="rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-1 py-0.5 text-[10px]"
				>
					<option value="asc">↑</option>
					<option value="desc">↓</option>
				</select>
			{/if}
		</div>
	{/if}

	{#if Object.keys(filters.text).length > 0 || Object.keys(filters.num).length > 0 || Object.keys(filters.date).length > 0 || filters.sort}
		<button class="text-[var(--color-muted)] underline hover:text-[var(--color-text)]" onclick={clearAll}>
			clear all
		</button>
	{/if}
</div>
