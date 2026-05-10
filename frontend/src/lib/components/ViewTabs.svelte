<script lang="ts">
	/**
	 * View tabs across the top of every entity list view. Includes a
	 * built-in "All" tab (no filter applied) followed by user-saved views,
	 * a "+ New view" button, and a per-tab menu (Rename / Delete).
	 */
	import { viewsStore } from '$lib/stores/views';
	import type { EntityType } from '$lib/types';
	import type { View } from '$lib/types/view';
	import Icon from './Icon.svelte';

	let {
		entityType,
		activeViewId,
		onSelect,
		onSaveCurrent,
		onUpdateActive,
		onActiveDeleted
	}: {
		entityType: EntityType;
		activeViewId: string | null;
		onSelect: (view: View | null) => void;
		onSaveCurrent: () => void;
		onUpdateActive: () => void;
		onActiveDeleted: () => void;
	} = $props();

	const views = $derived($viewsStore.file?.types[entityType]?.views ?? []);

	let menuOpenFor: string | null = $state(null);
	let renaming: View | null = $state(null);
	// Transient feedback for the "Save changes to view" button so the
	// user sees confirmation that the click took effect — otherwise
	// the change happens entirely silently and looks like a no-op.
	let saveState: 'idle' | 'saving' | 'saved' = $state('idle');

	async function handleUpdateActive() {
		if (saveState === 'saving') return;
		saveState = 'saving';
		try {
			await onUpdateActive();
			saveState = 'saved';
			setTimeout(() => {
				if (saveState === 'saved') saveState = 'idle';
			}, 1500);
		} catch {
			saveState = 'idle';
		}
	}

	// Same fix as DetailPanel's three-dot: ViewTabs is nested inside
	// the workspace grid (overflow-hidden) and the list section
	// (overflow-hidden), so an `absolute z-30` dropdown was clipped
	// underneath the list header. We compute the toggle's bounding
	// rect from the click event's currentTarget and render the menu
	// `position: fixed` at a `z-50` higher than any modal in the app.
	let menuPos: { top: number; right: number } = $state({ top: 0, right: 0 });
	let activeTrigger: HTMLButtonElement | null = null;

	function toggleMenu(viewId: string, trigger: HTMLButtonElement) {
		if (menuOpenFor === viewId) {
			menuOpenFor = null;
			activeTrigger = null;
			return;
		}
		const r = trigger.getBoundingClientRect();
		menuPos = { top: r.bottom + 4, right: window.innerWidth - r.right };
		activeTrigger = trigger;
		menuOpenFor = viewId;
	}

	const openView = $derived(menuOpenFor ? views.find((v) => v.id === menuOpenFor) ?? null : null);

	$effect(() => {
		if (!menuOpenFor) return;
		function onDocClick(e: MouseEvent) {
			const target = e.target as Node;
			if (activeTrigger?.contains(target)) return;
			menuOpenFor = null;
			activeTrigger = null;
		}
		function onKey(e: KeyboardEvent) {
			if (e.key === 'Escape') {
				menuOpenFor = null;
				activeTrigger = null;
			}
		}
		function onScrollOrResize() {
			menuOpenFor = null;
			activeTrigger = null;
		}
		const id = window.setTimeout(() => {
			window.addEventListener('click', onDocClick);
		}, 0);
		window.addEventListener('keydown', onKey);
		window.addEventListener('resize', onScrollOrResize);
		window.addEventListener('scroll', onScrollOrResize, true);
		return () => {
			window.clearTimeout(id);
			window.removeEventListener('click', onDocClick);
			window.removeEventListener('keydown', onKey);
			window.removeEventListener('resize', onScrollOrResize);
			window.removeEventListener('scroll', onScrollOrResize, true);
		};
	});

	async function deleteView(v: View) {
		if (!confirm(`Delete view "${v.name}"?`)) return;
		menuOpenFor = null;
		const wasActive = v.id === activeViewId;
		await viewsStore.delete(entityType, v.id);
		if (wasActive) onActiveDeleted();
	}

	// Drag-and-drop reorder. HTML5 drag API is enough here — light
	// touch since views.reorder already exists on the store. We track
	// the dragged id and the hover-target id so a thin marker shows
	// where the drop will land; on drop we commit a new order to the
	// backend (which atomically rewrites views.yaml).
	let draggingId: string | null = $state(null);
	let dropTargetId: string | null = $state(null);

	function onDragStart(event: DragEvent, v: View) {
		draggingId = v.id;
		if (event.dataTransfer) {
			event.dataTransfer.effectAllowed = 'move';
			event.dataTransfer.setData('text/plain', v.id);
		}
	}

	function onDragOver(event: DragEvent, v: View) {
		if (!draggingId || draggingId === v.id) return;
		event.preventDefault();
		if (event.dataTransfer) event.dataTransfer.dropEffect = 'move';
		dropTargetId = v.id;
	}

	function onDragLeave(v: View) {
		if (dropTargetId === v.id) dropTargetId = null;
	}

	async function onDrop(event: DragEvent, targetView: View) {
		event.preventDefault();
		const draggedId = draggingId;
		draggingId = null;
		dropTargetId = null;
		if (!draggedId || draggedId === targetView.id) return;
		const order = views.map((v) => v.id);
		const from = order.indexOf(draggedId);
		const to = order.indexOf(targetView.id);
		if (from === -1 || to === -1) return;
		order.splice(from, 1);
		order.splice(to, 0, draggedId);
		try {
			await viewsStore.reorder(entityType, order);
		} catch {
			/* viewsStore will surface the failure; the next render
			   reflects the unchanged on-disk order. */
		}
	}

	function onDragEnd() {
		draggingId = null;
		dropTargetId = null;
	}
</script>

<div class="flex items-center gap-1 overflow-x-auto border-b border-[var(--color-border)] px-2 pt-2 text-xs">
	<button
		class="flex items-center gap-1 whitespace-nowrap rounded-t border border-b-0 border-transparent px-3 py-1.5 hover:bg-white/5"
		class:active={activeViewId === null}
		onclick={() => onSelect(null)}
	>
		All
	</button>
	{#each views as v (v.id)}
		<div
			class="relative flex items-stretch"
			class:dragging={draggingId === v.id}
			class:drop-target={dropTargetId === v.id}
			role="group"
			ondragover={(e) => onDragOver(e, v)}
			ondragleave={() => onDragLeave(v)}
			ondrop={(e) => onDrop(e, v)}
		>
			<button
				class="flex items-center gap-1 whitespace-nowrap rounded-t border border-b-0 border-transparent px-3 py-1.5 hover:bg-white/5"
				class:active={activeViewId === v.id}
				onclick={() => onSelect(v)}
				draggable="true"
				ondragstart={(e) => onDragStart(e, v)}
				ondragend={onDragEnd}
				title="Drag to reorder"
			>
				{v.name}
			</button>
			{#if activeViewId === v.id}
				<button
					class="rounded-t border-b-0 px-1 py-1.5 text-[var(--color-muted)] hover:bg-white/5 hover:text-[var(--color-text)]"
					aria-label="View options"
					aria-haspopup="menu"
					aria-expanded={menuOpenFor === v.id}
					onclick={(e) => toggleMenu(v.id, e.currentTarget)}
				>
					⋯
				</button>
			{/if}
		</div>
	{/each}
	{#if activeViewId}
		<button
			class="ml-1 flex items-center gap-1 whitespace-nowrap rounded border px-2 py-1 transition-colors disabled:cursor-default"
			class:saved={saveState === 'saved'}
			class:idle={saveState !== 'saved'}
			disabled={saveState === 'saving'}
			onclick={handleUpdateActive}
			title="Overwrite this view with the current filter / sort / group / columns / view-mode"
		>
			{#if saveState === 'saved'}
				<Icon name="check" size={12} /> Saved
			{:else if saveState === 'saving'}
				Saving…
			{:else}
				<Icon name="check" size={12} /> Save changes to view
			{/if}
		</button>
	{/if}
	<button
		class="ml-1 flex items-center gap-1 whitespace-nowrap rounded border border-dashed border-[var(--color-border)] px-2 py-1 text-[var(--color-muted)] hover:bg-white/5 hover:text-[var(--color-text)]"
		onclick={onSaveCurrent}
		title="Save current filter / sort / group as a new view"
	>
		<Icon name="plus" size={12} /> New view
	</button>
</div>

{#if openView}
	<!-- See toggleMenu above — rendered `fixed` so the workspace
		 grid / list section's overflow:hidden can't clip the menu
		 underneath the list header. -->
	<div
		role="menu"
		class="fixed z-50 w-36 overflow-hidden rounded border border-[var(--color-border)] bg-[var(--color-panel)] text-xs shadow-xl"
		style="top: {menuPos.top}px; right: {menuPos.right}px"
	>
		<button
			class="block w-full border-b border-[var(--color-border)] px-3 py-1.5 text-left hover:bg-white/5"
			onclick={() => {
				renaming = openView;
				menuOpenFor = null;
				activeTrigger = null;
			}}
		>
			Rename
		</button>
		<button
			class="block w-full px-3 py-1.5 text-left hover:bg-red-900/30"
			onclick={() => deleteView(openView!)}
		>
			Delete
		</button>
	</div>
{/if}

{#if renaming}
	{#await import('./SaveViewDialog.svelte') then mod}
		{@const Dialog = mod.default}
		<Dialog
			{entityType}
			mode="rename"
			existing={renaming}
			viewKind={renaming.kind}
			filter={renaming.filter}
			sortKey={renaming.sort_key}
			sortDir={renaming.sort_dir}
			groupByKey={renaming.group_by_key}
			columns={renaming.columns}
			cardProperties={renaming.card_properties}
			onCancel={() => (renaming = null)}
			onSaved={() => (renaming = null)}
		/>
	{/await}
{/if}

<style>
	.active {
		background-color: rgba(108, 182, 255, 0.15);
		color: var(--color-accent);
		border-color: var(--color-border);
	}
	.idle {
		border-color: rgba(108, 182, 255, 0.4);
		background-color: rgba(108, 182, 255, 0.1);
		color: var(--color-accent);
	}
	.idle:hover {
		background-color: rgba(108, 182, 255, 0.2);
	}
	.saved {
		border-color: rgba(98, 196, 84, 0.4);
		background-color: rgba(98, 196, 84, 0.15);
		color: var(--color-good);
	}
	.dragging {
		opacity: 0.4;
	}
	.drop-target {
		box-shadow: inset 2px 0 0 var(--color-accent);
	}
</style>
