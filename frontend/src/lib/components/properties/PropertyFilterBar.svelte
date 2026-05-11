<script lang="ts">
	import Icon from '../Icon.svelte';
	import {
		COLOR_CLASSES,
		DEFAULT_PROPERTY_FILTERS,
		type Property,
		type PropertyFilters
	} from '$lib/types/schema';

	// Chip-only filter strip. Adding new filters now goes through the
	// "Compound filter" block below this bar; this component renders
	// the active chips (which still survive in the URL/saved-view
	// shape) and lets the user remove/edit them in-place.
	let {
		properties,
		filters = $bindable(DEFAULT_PROPERTY_FILTERS),
		onChange
	}: {
		properties: Property[];
		filters: PropertyFilters;
		onChange?: (next: PropertyFilters) => void;
	} = $props();

	function update(next: PropertyFilters) {
		filters = next;
		onChange?.(next);
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

	function clearAll() {
		update({ text: {}, num: {}, date: {} });
	}
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

	{#if Object.keys(filters.text).length > 0 || Object.keys(filters.num).length > 0 || Object.keys(filters.date).length > 0}
		<button class="ml-auto text-[var(--color-muted)] underline hover:text-[var(--color-text)]" onclick={clearAll}>
			clear all
		</button>
	{/if}
</div>
