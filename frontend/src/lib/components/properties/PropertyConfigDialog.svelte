<script lang="ts">
	import { schemaStore } from '$lib/stores/schema';
	import { ENTITY_TYPES, type EntityType } from '$lib/types';
	import {
		COLOR_PALETTE,
		COLOR_CLASSES,
		type OptionColor,
		type Property,
		type PropertyType,
		type SelectOption
	} from '$lib/types/schema';
	import Icon from '../Icon.svelte';

	let {
		entityType,
		mode,
		existing,
		onCancel,
		onSaved
	}: {
		entityType: EntityType;
		mode: 'add' | 'edit';
		existing: Property | null;
		onCancel: () => void;
		onSaved: () => void;
	} = $props();

	const PROPERTY_TYPES: { value: PropertyType; label: string }[] = [
		{ value: 'text', label: 'Text' },
		{ value: 'number', label: 'Number' },
		{ value: 'date', label: 'Date' },
		{ value: 'checkbox', label: 'Checkbox' },
		{ value: 'single-select', label: 'Single select' },
		{ value: 'multi-select', label: 'Multi-select' },
		{ value: 'url', label: 'URL' },
		{ value: 'relation', label: 'Relation' }
	];

	function slugify(s: string): string {
		return s
			.toLowerCase()
			.replace(/[^a-z0-9_]+/g, '_')
			.replace(/^_+|_+$/g, '')
			.replace(/^([0-9])/, '_$1') // ensure leading [a-z]
			.slice(0, 64);
	}

	// Snapshot prop values once at mount so $state initializers don't trip
	// Svelte's "state captures only the initial value" warning. The dialog is
	// remounted whenever a new property is opened — these don't need to react.
	const initial = (() => existing)();
	const initialMode = (() => mode)();

	let propType: PropertyType = $state(initial?.type ?? 'text');
	let key: string = $state(initial?.key ?? '');
	let keyManuallySet = $state(!!initial);
	let name: string = $state(initial?.name ?? '');
	let description: string = $state(initial?.description ?? '');
	let required: boolean = $state(initial?.required ?? false);
	let unit: string = $state(
		initial && initial.type === 'number' ? (initial.unit ?? '') : ''
	);
	let options: SelectOption[] = $state(
		initial && (initial.type === 'single-select' || initial.type === 'multi-select')
			? initial.options.map((o) => ({ ...o }))
			: []
	);
	let targetTypes: EntityType[] = $state(
		initial && initial.type === 'relation' ? (initial.target_types ?? []) : []
	);

	let saving = $state(false);
	let error: string | null = $state(null);

	const isEdit = initialMode === 'edit';
	const typeLocked = isEdit; // type is immutable after creation
	const keyLocked = isEdit;

	$effect(() => {
		if (!keyManuallySet) {
			key = slugify(name);
		}
	});

	function ensureValidKey(value: string): string | null {
		if (!/^[a-z][a-z0-9_]{0,63}$/.test(value)) {
			return 'Key must start with a letter and contain only lowercase letters, digits, or underscores.';
		}
		return null;
	}

	function ensureValidName(value: string): string | null {
		if (!value.trim()) return 'Name is required.';
		return null;
	}

	function addOption() {
		const usedColors = new Set(options.map((o) => o.color));
		const nextColor = COLOR_PALETTE.find((c) => !usedColors.has(c)) ?? 'gray';
		options = [...options, { value: '', label: '', color: nextColor }];
	}

	function removeOption(idx: number) {
		options = options.filter((_, i) => i !== idx);
	}

	function setOption(idx: number, patch: Partial<SelectOption>) {
		options = options.map((o, i) => (i === idx ? { ...o, ...patch } : o));
	}

	function buildPayload(): Property {
		const base = {
			key,
			name,
			description: description.trim() || undefined,
			required
		};
		switch (propType) {
			case 'text':
				return { ...base, type: 'text' };
			case 'number':
				return { ...base, type: 'number', unit: unit.trim() || undefined };
			case 'date':
				return { ...base, type: 'date' };
			case 'checkbox':
				return { ...base, type: 'checkbox' };
			case 'url':
				return { ...base, type: 'url' };
			case 'single-select':
				return { ...base, type: 'single-select', options };
			case 'multi-select':
				return { ...base, type: 'multi-select', options };
			case 'relation':
				return {
					...base,
					type: 'relation',
					target_types: targetTypes.length > 0 ? targetTypes : undefined
				};
		}
	}

	function buildPatch(): Parameters<typeof schemaStore.patchProperty>[2] {
		const patch: Record<string, unknown> = {
			name,
			description: description.trim() || null,
			required
		};
		if (propType === 'number') patch.unit = unit.trim() || null;
		if (propType === 'single-select' || propType === 'multi-select') patch.options = options;
		if (propType === 'relation' && targetTypes.length > 0) patch.target_types = targetTypes;
		return patch;
	}

	function validate(): string | null {
		const ne = ensureValidName(name);
		if (ne) return ne;
		if (!isEdit) {
			const ke = ensureValidKey(key);
			if (ke) return ke;
		}
		if (propType === 'single-select' || propType === 'multi-select') {
			if (options.length === 0) return 'At least one option is required.';
			for (const o of options) {
				if (!/^[a-z0-9][a-z0-9_-]{0,63}$/.test(o.value))
					return `Option value "${o.value}" is invalid.`;
				if (!o.label.trim()) return 'Option labels cannot be empty.';
			}
			const seen = new Set<string>();
			for (const o of options) {
				if (seen.has(o.value)) return `Duplicate option value: ${o.value}`;
				seen.add(o.value);
			}
		}
		return null;
	}

	async function save() {
		error = validate();
		if (error) return;
		saving = true;
		try {
			if (isEdit && existing) {
				await schemaStore.patchProperty(entityType, existing.key, buildPatch());
			} else {
				await schemaStore.addProperty(entityType, buildPayload());
			}
			onSaved();
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			saving = false;
		}
	}

	function toggleTargetType(t: EntityType) {
		targetTypes = targetTypes.includes(t)
			? targetTypes.filter((x) => x !== t)
			: [...targetTypes, t];
	}
</script>

<div class="fixed inset-0 z-40 flex items-center justify-center bg-black/70 p-6" role="dialog" aria-modal="true">
	<div class="flex max-h-[90vh] w-[min(560px,92vw)] flex-col overflow-hidden rounded-lg border border-[var(--color-border)] bg-[var(--color-panel)] shadow-2xl">
		<header class="flex items-center justify-between border-b border-[var(--color-border)] px-4 py-3">
			<h2 class="text-sm font-semibold">
				{isEdit ? `Edit "${existing?.name}"` : 'Add property'}
			</h2>
			<button class="rounded p-1 hover:bg-white/5" onclick={onCancel} aria-label="Close">
				<Icon name="x" />
			</button>
		</header>

		<div class="flex flex-1 flex-col gap-4 overflow-auto p-4">
			<label class="grid gap-1">
				<span class="text-xs font-medium">Name</span>
				<input
					bind:value={name}
					class="rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-3 py-2 text-sm outline-none focus:border-[var(--color-accent)]"
				/>
			</label>

			<label class="grid gap-1">
				<span class="text-xs font-medium">Key {keyLocked ? '(locked)' : ''}</span>
				<input
					bind:value={key}
					oninput={() => (keyManuallySet = true)}
					readonly={keyLocked}
					class="rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-3 py-2 text-sm outline-none focus:border-[var(--color-accent)] disabled:opacity-60"
				/>
				<span class="text-[10px] text-[var(--color-muted)]">
					YAML key. Auto-generated from the name; click to override. Immutable after creation.
				</span>
			</label>

			<label class="grid gap-1">
				<span class="text-xs font-medium">Type {typeLocked ? '(locked)' : ''}</span>
				<select
					bind:value={propType}
					disabled={typeLocked}
					class="rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-3 py-2 text-sm text-[var(--color-text)] outline-none focus:border-[var(--color-accent)] disabled:opacity-60"
				>
					{#each PROPERTY_TYPES as opt}
						<option value={opt.value}>{opt.label}</option>
					{/each}
				</select>
			</label>

			<label class="grid gap-1">
				<span class="text-xs font-medium">Description (optional)</span>
				<textarea
					bind:value={description}
					rows="2"
					class="rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-3 py-2 text-sm outline-none focus:border-[var(--color-accent)]"
				></textarea>
			</label>

			<label class="inline-flex items-center gap-2 text-sm">
				<input type="checkbox" bind:checked={required} />
				<span>Required</span>
			</label>

			{#if propType === 'number'}
				<label class="grid gap-1">
					<span class="text-xs font-medium">Unit (display-only)</span>
					<input
						bind:value={unit}
						placeholder="e.g. /4.0, %, kg"
						class="rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-3 py-2 text-sm outline-none focus:border-[var(--color-accent)]"
					/>
				</label>
			{/if}

			{#if propType === 'single-select' || propType === 'multi-select'}
				<div class="flex flex-col gap-2">
					<div class="flex items-center justify-between">
						<span class="text-xs font-medium">Options</span>
						<button class="rounded border border-[var(--color-border)] px-2 py-1 text-[10px] hover:bg-white/5" onclick={addOption}>
							<Icon name="plus" /> Add option
						</button>
					</div>
					{#each options as opt, idx}
						<div class="grid grid-cols-[1fr_1fr_auto_auto] items-center gap-2">
							<input
								placeholder="value"
								value={opt.value}
								oninput={(e) => setOption(idx, { value: (e.currentTarget as HTMLInputElement).value })}
								readonly={isEdit && existing && (existing.type === 'single-select' || existing.type === 'multi-select') && (existing.options.find((o) => o.value === opt.value)?.value === opt.value && options[idx].value === opt.value)}
								class="rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-2 py-1 text-sm outline-none"
							/>
							<input
								placeholder="label"
								value={opt.label}
								oninput={(e) => setOption(idx, { label: (e.currentTarget as HTMLInputElement).value })}
								class="rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-2 py-1 text-sm outline-none"
							/>
							<select
								value={opt.color}
								onchange={(e) =>
									setOption(idx, { color: (e.currentTarget as HTMLSelectElement).value as OptionColor })}
								class="rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-2 py-1 text-xs text-[var(--color-text)] outline-none"
							>
								{#each COLOR_PALETTE as c}
									<option value={c}>{c}</option>
								{/each}
							</select>
							<button class="rounded p-1 hover:bg-white/5" aria-label="Remove option" onclick={() => removeOption(idx)}>
								<Icon name="x" size={14} />
							</button>
						</div>
					{/each}
					<p class="text-[10px] text-[var(--color-muted)]">
						Option <em>values</em> are immutable once created (renaming would orphan entity values). Labels and colors are editable.
					</p>
				</div>
			{/if}

			{#if propType === 'relation'}
				<div class="flex flex-col gap-2">
					<span class="text-xs font-medium">Target types (empty = any)</span>
					<div class="flex flex-wrap gap-1.5">
						{#each ENTITY_TYPES as t}
							{@const active = targetTypes.includes(t)}
							<button
								class="rounded border px-2 py-0.5 text-xs"
								class:active
								onclick={() => toggleTargetType(t)}
							>
								{t}
							</button>
						{/each}
					</div>
				</div>
			{/if}

			{#if error}
				<div class="rounded border border-[var(--color-bad)] bg-red-900/20 px-3 py-2 text-xs text-[var(--color-bad)]">
					{error}
				</div>
			{/if}
		</div>

		<footer class="flex justify-end gap-2 border-t border-[var(--color-border)] px-4 py-3">
			<button class="rounded border border-[var(--color-border)] px-3 py-1.5 text-xs hover:bg-white/5" onclick={onCancel}>
				Cancel
			</button>
			<button
				class="rounded border border-blue-700 bg-blue-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-blue-700 disabled:opacity-50"
				disabled={saving}
				onclick={save}
			>
				{saving ? 'Saving…' : isEdit ? 'Save changes' : 'Add property'}
			</button>
		</footer>
	</div>
</div>

<style>
	.active {
		background-color: rgba(108, 182, 255, 0.18);
		color: var(--color-accent);
		border-color: var(--color-accent);
	}
	button {
		border-color: var(--color-border);
	}
</style>
