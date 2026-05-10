<script lang="ts">
	/**
	 * Captures the current list-view state (filter / sort / group / view-mode
	 * / column visibility / card properties) into a named saved view.
	 */
	import { viewsStore } from '$lib/stores/views';
	import type { EntityType } from '$lib/types';
	import type { ViewKind, View } from '$lib/types/view';
	import type { ViewFilter } from '$lib/types/view';
	import Icon from './Icon.svelte';

	let {
		entityType,
		mode,
		existing,
		viewKind,
		filter,
		sortKey,
		sortDir,
		groupByKey,
		columns,
		cardProperties,
		onCancel,
		onSaved
	}: {
		entityType: EntityType;
		mode: 'create' | 'rename';
		existing?: View | null;
		viewKind: ViewKind;
		filter: ViewFilter;
		sortKey?: string | null;
		sortDir?: 'asc' | 'desc';
		groupByKey?: string | null;
		columns?: string[] | null;
		cardProperties?: string[] | null;
		onCancel: () => void;
		onSaved: (view: View) => void;
	} = $props();

	const initialName = (() => existing?.name ?? '')();
	let name = $state(initialName);

	function autofocus(node: HTMLElement) {
		// Defer to next tick so the dialog is mounted before focus.
		queueMicrotask(() => node.focus());
	}
	let saving = $state(false);
	let error: string | null = $state(null);

	async function save() {
		const trimmed = name.trim();
		if (!trimmed) {
			error = 'Name is required.';
			return;
		}
		saving = true;
		try {
			let view: View;
			if (mode === 'rename' && existing) {
				view = await viewsStore.update(entityType, existing.id, {
					name: trimmed,
					kind: existing.kind,
					filter: existing.filter,
					sort_key: existing.sort_key,
					sort_dir: existing.sort_dir,
					group_by_key: existing.group_by_key,
					columns: existing.columns,
					card_properties: existing.card_properties
				});
			} else {
				view = await viewsStore.create(entityType, {
					name: trimmed,
					kind: viewKind,
					filter,
					sort_key: sortKey ?? null,
					sort_dir: sortDir ?? 'asc',
					group_by_key: groupByKey ?? null,
					columns: columns ?? null,
					card_properties: cardProperties ?? null
				});
			}
			onSaved(view);
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			saving = false;
		}
	}

	function onKey(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			event.preventDefault();
			onCancel();
		}
	}
</script>

<svelte:window onkeydown={onKey} />

<div class="fixed inset-0 z-40 flex items-center justify-center bg-black/70 p-6" role="dialog" aria-modal="true">
	<div class="flex w-[min(420px,92vw)] flex-col overflow-hidden rounded-lg border border-[var(--color-border)] bg-[var(--color-panel)] shadow-2xl">
		<header class="flex items-center justify-between border-b border-[var(--color-border)] px-4 py-3">
			<h2 class="text-sm font-semibold">
				{mode === 'rename' ? 'Rename view' : 'Save current view'}
			</h2>
			<button class="rounded p-1 hover:bg-white/5" onclick={onCancel} aria-label="Close">
				<Icon name="x" />
			</button>
		</header>

		<div class="flex flex-col gap-3 p-4">
			<label class="grid gap-1">
				<span class="text-xs font-medium">Name</span>
				<input
					bind:value={name}
					placeholder="e.g. Reach schools"
					use:autofocus
					class="rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-3 py-2 text-sm outline-none focus:border-[var(--color-accent)]"
				/>
			</label>
			{#if mode !== 'rename'}
				<p class="text-[10px] text-[var(--color-muted)]">
					Captures the current view mode ({viewKind}), active filters, sort, group-by, and column visibility.
					You can edit the saved view later by changing its tab and clicking save again.
				</p>
			{/if}
			{#if error}
				<div class="rounded border border-[var(--color-bad)] bg-red-900/20 px-2 py-1 text-xs text-[var(--color-bad)]">
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
				disabled={saving || !name.trim()}
				onclick={save}
			>
				{saving ? 'Saving…' : mode === 'rename' ? 'Save' : 'Create view'}
			</button>
		</footer>
	</div>
</div>
