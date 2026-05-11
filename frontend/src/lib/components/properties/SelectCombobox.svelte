<script lang="ts">
	/**
	 * Notion-style typeahead combobox for single- or multi-select
	 * properties. Filters the option list as the user types and shows
	 * a "Create 'X'" affordance when no exact match exists — this is
	 * the Phase-E inline-option-creation surface.
	 *
	 * The parent owns the create policy: pass an `onCreate` callback
	 * that mutates the schema (e.g. via `schemaStore.patchProperty`)
	 * and returns the new SelectOption. If `onCreate` is omitted the
	 * Create-X row is hidden — the combobox becomes a strict picker.
	 */
	import type { SelectOption } from '$lib/types/schema';
	import { COLOR_CLASSES } from '$lib/types/schema';

	type Props = {
		value: string | string[];
		multi?: boolean;
		options: SelectOption[];
		placeholder?: string;
		disabled?: boolean;
		onChange: (next: string | string[]) => void;
		/** Called when the user picks "Create 'X'". Should persist the
		 * new option (typically by patching the schema) and return the
		 * created SelectOption. Returning a rejected promise leaves the
		 * dropdown open and surfaces the error to the parent. */
		onCreate?: (label: string) => Promise<SelectOption>;
	};

	let {
		value,
		multi = false,
		options,
		placeholder = 'Select…',
		disabled = false,
		onChange,
		onCreate
	}: Props = $props();

	let open = $state(false);
	let query = $state('');
	let creating = $state(false);
	let createError: string | null = $state(null);
	let inputEl: HTMLInputElement | undefined = $state();
	let highlight = $state(0);
	const listboxId = `combobox-list-${Math.random().toString(36).slice(2, 9)}`;

	const selectedValues = $derived(
		Array.isArray(value) ? value : value ? [value] : []
	);
	const selectedOptions = $derived(
		selectedValues.map((v) => options.find((o) => o.value === v)).filter((o): o is SelectOption => !!o)
	);
	const orphanValues = $derived(
		selectedValues.filter((v) => !options.some((o) => o.value === v))
	);

	const filtered = $derived(
		query.trim().length === 0
			? options
			: options.filter((o) => {
					const q = query.toLowerCase();
					return o.label.toLowerCase().includes(q) || o.value.toLowerCase().includes(q);
				})
	);

	const queryTrimmed = $derived(query.trim());
	const exactMatch = $derived(
		queryTrimmed.length > 0 &&
			options.some(
				(o) =>
					o.label.toLowerCase() === queryTrimmed.toLowerCase() ||
					o.value.toLowerCase() === queryTrimmed.toLowerCase()
			)
	);
	const showCreateRow = $derived(!!onCreate && queryTrimmed.length > 0 && !exactMatch);

	function toggle(opt: SelectOption) {
		if (multi) {
			const arr = selectedValues.includes(opt.value)
				? selectedValues.filter((v) => v !== opt.value)
				: [...selectedValues, opt.value];
			onChange(arr);
			query = '';
			highlight = 0;
		} else {
			onChange(opt.value);
			open = false;
			query = '';
		}
	}

	function clearOne(v: string) {
		if (multi) {
			onChange(selectedValues.filter((x) => x !== v));
		} else {
			onChange('');
		}
	}

	async function doCreate() {
		if (!onCreate || !queryTrimmed) return;
		creating = true;
		createError = null;
		try {
			const created = await onCreate(queryTrimmed);
			if (multi) {
				onChange([...selectedValues, created.value]);
			} else {
				onChange(created.value);
				open = false;
			}
			query = '';
			highlight = 0;
		} catch (e) {
			createError = e instanceof Error ? e.message : String(e);
		} finally {
			creating = false;
		}
	}

	function onKey(e: KeyboardEvent) {
		const total = filtered.length + (showCreateRow ? 1 : 0);
		if (e.key === 'ArrowDown') {
			e.preventDefault();
			open = true;
			highlight = total === 0 ? 0 : (highlight + 1) % total;
		} else if (e.key === 'ArrowUp') {
			e.preventDefault();
			highlight = total === 0 ? 0 : (highlight - 1 + total) % total;
		} else if (e.key === 'Enter') {
			e.preventDefault();
			if (highlight < filtered.length) {
				toggle(filtered[highlight]);
			} else if (showCreateRow) {
				void doCreate();
			}
		} else if (e.key === 'Escape') {
			e.preventDefault();
			open = false;
		} else if (e.key === 'Backspace' && query === '' && multi && selectedValues.length > 0) {
			onChange(selectedValues.slice(0, -1));
		}
	}

	function onBlur() {
		// Defer so a click on a dropdown row registers before the
		// dropdown closes.
		setTimeout(() => {
			open = false;
		}, 120);
	}
</script>

<div class="relative">
	<div
		role="combobox"
		aria-expanded={open}
		aria-haspopup="listbox"
		aria-controls={listboxId}
		tabindex="-1"
		class="flex flex-wrap items-center gap-1 rounded border bg-[var(--color-bg)] px-2 py-1.5 text-sm {open
			? 'border-[var(--color-accent)]'
			: 'border-[var(--color-border)]'} {disabled ? 'opacity-50' : ''}"
		onclick={() => {
			if (disabled) return;
			open = true;
			inputEl?.focus();
		}}
		onkeydown={(e) => {
			// The actual input below owns key handling; this handler
			// just satisfies the a11y_click_events_have_key_events lint
			// for the wrapping clickable container.
			if (e.key === 'Enter' || e.key === ' ') {
				inputEl?.focus();
			}
		}}
	>
		{#each selectedOptions as opt}
			<span
				class="inline-flex items-center gap-1 rounded border px-1.5 py-0.5 text-xs {COLOR_CLASSES[opt.color].bg} {COLOR_CLASSES[opt.color].text} {COLOR_CLASSES[opt.color].border}"
			>
				{opt.label}
				{#if !disabled}
					<button
						type="button"
						class="text-current opacity-60 hover:opacity-100"
						onclick={(e) => {
							e.stopPropagation();
							clearOne(opt.value);
						}}
						aria-label="Remove {opt.label}">×</button
					>
				{/if}
			</span>
		{/each}
		{#each orphanValues as v}
			<span
				class="inline-flex items-center gap-1 rounded border border-[var(--color-bad)]/40 bg-[var(--color-bad)]/10 px-1.5 py-0.5 text-xs text-[var(--color-bad)]"
				title="Value not in the option list — orphaned"
			>
				{v}
				{#if !disabled}
					<button
						type="button"
						class="opacity-60 hover:opacity-100"
						onclick={(e) => {
							e.stopPropagation();
							clearOne(v);
						}}
						aria-label="Remove orphan {v}">×</button
					>
				{/if}
			</span>
		{/each}
		<input
			bind:this={inputEl}
			bind:value={query}
			onkeydown={onKey}
			onfocus={() => (open = true)}
			onblur={onBlur}
			{disabled}
			placeholder={selectedValues.length === 0 ? placeholder : ''}
			class="min-w-[60px] flex-1 bg-transparent px-1 py-0.5 outline-none"
		/>
	</div>

	{#if open && !disabled}
		<div
			id={listboxId}
			role="listbox"
			class="absolute z-30 mt-1 max-h-64 w-full min-w-[220px] overflow-auto rounded border border-[var(--color-border)] bg-[var(--color-panel)] shadow-lg"
		>
			{#each filtered as opt, i}
				{@const isSelected = selectedValues.includes(opt.value)}
				<button
					type="button"
					class="flex w-full items-center justify-between gap-2 px-2 py-1.5 text-left text-sm hover:bg-white/5 {i ===
					highlight
						? 'bg-white/5'
						: ''}"
					onmousedown={(e) => {
						e.preventDefault();
						toggle(opt);
					}}
					onmouseenter={() => (highlight = i)}
				>
					<span
						class="inline-flex items-center rounded border px-1.5 py-0.5 text-xs {COLOR_CLASSES[opt.color].bg} {COLOR_CLASSES[opt.color].text} {COLOR_CLASSES[opt.color].border}"
					>
						{opt.label}
					</span>
					{#if isSelected}
						<span class="text-xs text-[var(--color-muted)]">selected</span>
					{/if}
				</button>
			{:else}
				{#if !showCreateRow}
					<div class="px-2 py-2 text-xs text-[var(--color-muted)]">No options.</div>
				{/if}
			{/each}

			{#if showCreateRow}
				<button
					type="button"
					class="flex w-full items-center gap-2 border-t border-[var(--color-border)] px-2 py-1.5 text-left text-sm hover:bg-white/5 {filtered.length ===
					highlight
						? 'bg-white/5'
						: ''}"
					onmousedown={(e) => {
						e.preventDefault();
						void doCreate();
					}}
					disabled={creating}
				>
					<span class="text-[var(--color-muted)]">Create</span>
					<span
						class="inline-flex items-center rounded border border-[var(--color-border)] bg-white/5 px-1.5 py-0.5 text-xs"
					>
						{queryTrimmed}
					</span>
					{#if creating}
						<span class="text-xs text-[var(--color-muted)]">…</span>
					{/if}
				</button>
			{/if}

			{#if createError}
				<div class="border-t border-[var(--color-border)] px-2 py-1 text-xs text-[var(--color-bad)]">
					{createError}
				</div>
			{/if}
		</div>
	{/if}
</div>
