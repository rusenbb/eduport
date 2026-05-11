<script lang="ts">
	/**
	 * Notion-style compound filter builder (Phase B).
	 *
	 * One group level: a top-level AND/OR combinator with a flat list
	 * of conditions. Each condition picks a property, an operator, and
	 * (for non-unary operators) a value editor whose shape depends on
	 * the property's type.
	 *
	 * The component is controlled — it emits the new tree on every
	 * change via `onChange`. Empty trees are returned as `null` so the
	 * caller can clear the saved view's `filter_tree` cleanly.
	 */
	import type { Property } from '$lib/types/schema';
	import {
		defaultOperatorFor,
		emptyTree,
		operatorsFor,
		type FilterCondition,
		type FilterGroup,
		type FilterNode,
		type FilterOperator,
		type FilterTree,
		type FilterValue
	} from '$lib/types/filter';

	type Props = {
		tree: FilterTree | null | undefined;
		properties: Property[];
		onChange: (next: FilterTree | null) => void;
	};

	let { tree, properties, onChange }: Props = $props();

	// Normalise to a single top-level group. Anything else (nested
	// groups, naked condition at the root) is collapsed into the
	// top-level group's children for editing — no data loss because
	// we serialise back as a group at the end.
	const group = $derived<FilterGroup>(rootAsGroup(tree));

	function rootAsGroup(t: FilterTree | null | undefined): FilterGroup {
		if (!t?.root) return { kind: 'group', op: 'and', children: [] };
		if (t.root.kind === 'group') return t.root;
		return { kind: 'group', op: 'and', children: [t.root] };
	}

	function emit(next: FilterGroup) {
		const trimmed = next.children.length === 0 ? null : ({ root: next } satisfies FilterTree);
		onChange(trimmed);
	}

	function setCombinator(op: 'and' | 'or') {
		emit({ ...group, op });
	}

	function addCondition() {
		const firstProp = properties[0];
		if (!firstProp) return;
		const op = defaultOperatorFor(firstProp.type);
		const cond: FilterCondition = {
			kind: 'cond',
			property_key: firstProp.key,
			operator: op,
			value: defaultValueFor(firstProp, op)
		};
		emit({ ...group, children: [...group.children, cond] });
	}

	function removeAt(i: number) {
		const next = group.children.slice();
		next.splice(i, 1);
		emit({ ...group, children: next });
	}

	function updateAt(i: number, patch: Partial<FilterCondition>) {
		const next = group.children.slice();
		const cur = next[i];
		if (cur.kind !== 'cond') return;
		next[i] = { ...cur, ...patch };
		emit({ ...group, children: next });
	}

	function changeProperty(i: number, propertyKey: string) {
		const prop = properties.find((p) => p.key === propertyKey);
		if (!prop) return;
		const op = defaultOperatorFor(prop.type);
		updateAt(i, {
			property_key: propertyKey,
			operator: op,
			value: defaultValueFor(prop, op)
		});
	}

	function changeOperator(i: number, op: FilterOperator) {
		const cond = group.children[i];
		if (cond.kind !== 'cond') return;
		const prop = properties.find((p) => p.key === cond.property_key);
		const meta = prop ? operatorsFor(prop.type).find((o) => o.op === op) : undefined;
		updateAt(i, {
			operator: op,
			value: meta?.hasValue && prop ? defaultValueFor(prop, op) : undefined
		});
	}

	function changeValue(i: number, value: FilterValue) {
		updateAt(i, { value });
	}

	function defaultValueFor(prop: Property, op: FilterOperator): FilterValue | undefined {
		const meta = operatorsFor(prop.type).find((o) => o.op === op);
		if (!meta?.hasValue) return undefined;
		switch (prop.type) {
			case 'number':
				return 0;
			case 'checkbox':
				return true;
			case 'multi-select':
				return [] as string[];
			case 'single-select':
				// single-select equals expects a value; use first option if any
				if (op === 'contains_any' || op === 'does_not_contain') return [];
				return prop.options[0]?.value ?? '';
			default:
				return '';
		}
	}

	function clearAll() {
		emit({ kind: 'group', op: 'and', children: [] });
	}

	const propertyMap = $derived(new Map(properties.map((p) => [p.key, p])));
</script>

<div class="rounded border border-[var(--color-border)] bg-[var(--color-panel)] p-3">
	<div class="mb-2 flex items-center justify-between">
		<div class="flex items-center gap-2">
			<span class="text-xs uppercase tracking-wider text-[var(--color-muted)]">Filter</span>
			{#if group.children.length > 1}
				<div class="flex rounded border border-[var(--color-border)] text-xs">
					<button
						type="button"
						class="px-2 py-0.5 {group.op === 'and'
							? 'bg-[var(--color-accent)]/20 text-[var(--color-text)]'
							: 'text-[var(--color-muted)]'}"
						onclick={() => setCombinator('and')}>AND</button
					>
					<button
						type="button"
						class="px-2 py-0.5 {group.op === 'or'
							? 'bg-[var(--color-accent)]/20 text-[var(--color-text)]'
							: 'text-[var(--color-muted)]'}"
						onclick={() => setCombinator('or')}>OR</button
					>
				</div>
			{/if}
		</div>
		{#if group.children.length > 0}
			<button
				type="button"
				class="text-xs text-[var(--color-muted)] hover:text-[var(--color-text)]"
				onclick={clearAll}>Clear all</button
			>
		{/if}
	</div>

	<div class="flex flex-col gap-2">
		{#each group.children as child, i (i)}
			{#if child.kind === 'cond'}
				{@const prop = propertyMap.get(child.property_key)}
				<div class="flex flex-wrap items-center gap-2 rounded bg-white/[0.02] px-2 py-1.5 text-sm">
					{#if i > 0}
						<span class="text-[10px] uppercase text-[var(--color-muted)]">{group.op}</span>
					{/if}
					<select
						value={child.property_key}
						onchange={(e) => changeProperty(i, e.currentTarget.value)}
						class="rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-2 py-1 text-xs"
					>
						{#each properties as p}
							<option value={p.key}>{p.name}</option>
						{/each}
					</select>
					{#if prop}
						<select
							value={child.operator}
							onchange={(e) =>
								changeOperator(i, e.currentTarget.value as FilterOperator)}
							class="rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-2 py-1 text-xs"
						>
							{#each operatorsFor(prop.type) as op}
								<option value={op.op}>{op.label}</option>
							{/each}
						</select>
						{#if operatorsFor(prop.type).find((o) => o.op === child.operator)?.hasValue}
							{@const opName = child.operator}
							{#if prop.type === 'number'}
								<input
									type="number"
									step="any"
									value={typeof child.value === 'number' ? child.value : 0}
									oninput={(e) =>
										changeValue(i, Number(e.currentTarget.value))}
									class="w-28 rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-2 py-1 text-xs"
								/>
							{:else if prop.type === 'date'}
								<input
									type="date"
									value={typeof child.value === 'string' ? child.value : ''}
									oninput={(e) =>
										changeValue(i, e.currentTarget.value)}
									class="rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-2 py-1 text-xs"
								/>
							{:else if prop.type === 'checkbox'}
								<select
									value={child.value === true ? 'true' : 'false'}
									onchange={(e) => changeValue(i, e.currentTarget.value === 'true')}
									class="rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-2 py-1 text-xs"
								>
									<option value="true">checked</option>
									<option value="false">unchecked</option>
								</select>
							{:else if prop.type === 'single-select'}
								<select
									value={typeof child.value === 'string' ? child.value : ''}
									onchange={(e) => changeValue(i, e.currentTarget.value)}
									class="rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-2 py-1 text-xs"
								>
									{#each prop.options as opt}
										<option value={opt.value}>{opt.label}</option>
									{/each}
								</select>
							{:else if prop.type === 'multi-select' && (opName === 'contains_any' || opName === 'does_not_contain')}
								<div class="flex flex-wrap gap-1">
									{#each prop.options as opt}
										{@const arr = Array.isArray(child.value) ? child.value : []}
										{@const checked = arr.includes(opt.value)}
										<button
											type="button"
											onclick={() => {
												const next = checked
													? arr.filter((v) => v !== opt.value)
													: [...arr, opt.value];
												changeValue(i, next);
											}}
											class="rounded border px-1.5 py-0.5 text-xs {checked
												? 'border-[var(--color-accent)] bg-[var(--color-accent)]/20'
												: 'border-[var(--color-border)] bg-[var(--color-bg)] text-[var(--color-muted)]'}"
										>
											{opt.label}
										</button>
									{/each}
								</div>
							{:else}
								<input
									type="text"
									value={typeof child.value === 'string' ? child.value : ''}
									oninput={(e) => changeValue(i, e.currentTarget.value)}
									class="w-40 rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-2 py-1 text-xs"
								/>
							{/if}
						{/if}
					{/if}
					<button
						type="button"
						onclick={() => removeAt(i)}
						class="ml-auto text-[var(--color-muted)] hover:text-[var(--color-bad)]"
						aria-label="Remove condition">×</button
					>
				</div>
			{/if}
		{/each}

		<button
			type="button"
			onclick={addCondition}
			disabled={properties.length === 0}
			class="self-start rounded border border-dashed border-[var(--color-border)] px-2 py-1 text-xs text-[var(--color-muted)] hover:border-[var(--color-accent)] hover:text-[var(--color-text)]"
		>
			+ Add condition
		</button>
	</div>
</div>
