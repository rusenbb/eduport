<script lang="ts">
	/**
	 * One table cell. Read-only render that swaps to inline-edit on click,
	 * driven by an `editing` flag the parent owns. Saves via `onSave(next)`,
	 * cancels via `onCancel`.
	 *
	 * Built-in columns (FieldDef-shaped) are rendered read-only — clicking
	 * them opens the parent row's detail panel rather than inline-editing.
	 */
	import type { FieldDef } from '$lib/entities/meta';
	import type { Property, ValueWarning } from '$lib/types/schema';
	import PropertyEditor from './PropertyEditor.svelte';
	import PropertyValue from './PropertyValue.svelte';
	import PropertyWarningChip from './PropertyWarningChip.svelte';

	let {
		column,
		value,
		warnings = [],
		editing,
		onStartEdit,
		onSave,
		onCancel
	}: {
		column: { kind: 'custom'; prop: Property } | { kind: 'builtin'; field: FieldDef };
		value: unknown;
		warnings?: ValueWarning[];
		editing: boolean;
		onStartEdit: () => void;
		onSave: (next: unknown) => void;
		onCancel: () => void;
	} = $props();

	// `draft` mirrors `value` while editing. We re-seed it whenever the cell
	// transitions from read-only to editing so external changes (saved by
	// another row, watcher refresh) are reflected when the user clicks in.
	let draft: unknown = $state(undefined);

	$effect(() => {
		if (editing) draft = value;
	});

	function commitOnEnter(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			onSave(draft);
		} else if (e.key === 'Escape') {
			e.preventDefault();
			onCancel();
		}
	}

	function renderBuiltin(field: FieldDef, v: unknown) {
		if (v === undefined || v === null || v === '') return null;
		switch (field.kind) {
			case 'url':
				return { kind: 'url', text: String(v) } as const;
			case 'email':
				return { kind: 'email', text: String(v) } as const;
			case 'wikilink':
				return { kind: 'wikilink', text: String(v) } as const;
			case 'wikilinks':
				return { kind: 'wikilinks', text: Array.isArray(v) ? (v as string[]).length : 0 } as const;
			case 'resources':
				return { kind: 'resources', text: Array.isArray(v) ? (v as unknown[]).length : 0 } as const;
			case 'date':
				return { kind: 'text', text: String(v) } as const;
			case 'select':
				return { kind: 'badge', text: String(v) } as const;
			default:
				return { kind: 'text', text: String(v) } as const;
		}
	}
</script>

<div
	class="flex min-h-[28px] flex-wrap items-center gap-1 px-2 py-1 text-xs hover:bg-white/[0.03]"
	role="button"
	tabindex="0"
	onclick={(e) => {
		if (column.kind === 'custom' && !editing) {
			e.stopPropagation();
			onStartEdit();
		}
	}}
	onkeydown={(e) => {
		if (column.kind === 'custom' && !editing && (e.key === 'Enter' || e.key === ' ')) {
			e.preventDefault();
			e.stopPropagation();
			onStartEdit();
		}
	}}
>
	{#if column.kind === 'custom'}
		{#if editing}
			<div
				class="min-w-0 flex-1"
				onkeydown={commitOnEnter}
				onblur={() => onSave(draft)}
				role="textbox"
				tabindex="-1"
			>
				<PropertyEditor prop={column.prop} bind:value={draft} />
			</div>
			<button
				class="rounded border border-[var(--color-accent)] bg-[var(--color-accent)]/15 px-1.5 py-0.5 text-[10px] text-[var(--color-accent)] hover:bg-[var(--color-accent)]/25"
				onclick={(e) => {
					e.stopPropagation();
					onSave(draft);
				}}
			>
				Save
			</button>
			<button
				class="rounded border border-[var(--color-border)] px-1.5 py-0.5 text-[10px] hover:bg-white/5"
				onclick={(e) => {
					e.stopPropagation();
					onCancel();
				}}
			>
				Cancel
			</button>
		{:else}
			<PropertyValue prop={column.prop} {value} />
			{#each warnings as w}
				<PropertyWarningChip warning={w} />
			{/each}
		{/if}
	{:else}
		{@const r = renderBuiltin(column.field, value)}
		{#if r === null}
			<span class="italic text-[var(--color-muted)]">—</span>
		{:else if r.kind === 'url'}
			<a class="truncate text-[var(--color-accent)] hover:opacity-80" href={r.text} target="_blank" rel="noreferrer">
				{r.text}
			</a>
		{:else if r.kind === 'email'}
			<span class="truncate">{r.text}</span>
		{:else if r.kind === 'wikilink'}
			<span class="rounded border border-[var(--color-border)] bg-white/5 px-1.5 py-0.5 text-[10px]">
				{r.text}
			</span>
		{:else if r.kind === 'wikilinks' || r.kind === 'resources'}
			<span class="text-[var(--color-muted)]">{r.text} item{r.text === 1 ? '' : 's'}</span>
		{:else if r.kind === 'badge'}
			<span class="rounded border border-[var(--color-border)] bg-white/5 px-1.5 py-0.5 text-[10px]">
				{r.text}
			</span>
		{:else}
			<span class="truncate">{r.text}</span>
		{/if}
	{/if}
</div>
