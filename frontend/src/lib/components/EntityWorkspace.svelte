<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { getContext, onMount } from 'svelte';
	import type { Writable } from 'svelte/store';
	import { deleteEntity, getEntity, listEntities } from '$lib/api/entities';
	import { CoreCommandError, listenCoreEvent } from '$lib/api/client';
	import {
		filterEntitiesByProperties,
		hasActiveFilters,
		parseFilterParams,
		writeFilterParams
	} from '$lib/api/schema';
	import { filters } from '$lib/stores/filters';
	import { schemaStore } from '$lib/stores/schema';
	import { settings } from '$lib/stores/settings';
	import { toasts } from '$lib/stores/toasts';
	import type { EntityDetail, EntityListItem, EntityType } from '$lib/types';
	import { DEFAULT_PROPERTY_FILTERS, type PropertyFilters } from '$lib/types/schema';
	import { TYPE_LABELS, builtinFilterableProperties } from '$lib/entities/meta';
	import { confirmDestructive, isTauri, revealInFileManager } from '$lib/tauri';
	import { isTypingTarget } from '$lib/keyboard';
	import EntityBodyEditor from './EntityBodyEditor.svelte';
	import DetailPanel from './DetailPanel.svelte';
	import EntityForm from './EntityForm.svelte';
	import EntityList from './EntityList.svelte';
	import EntityListSkeleton from './EntityListSkeleton.svelte';
	import EmailGroupedList from './EmailGroupedList.svelte';
	import GroupedList from './GroupedList.svelte';
	import KanbanBoard from './KanbanBoard.svelte';
	import SaveViewDialog from './SaveViewDialog.svelte';
	import TableView from './TableView.svelte';
	import ViewTabs from './ViewTabs.svelte';
	import CardPropertiesMenu from './properties/CardPropertiesMenu.svelte';
	import ColumnVisibilityMenu from './properties/ColumnVisibilityMenu.svelte';
	import PropertyFilterBar from './properties/PropertyFilterBar.svelte';
	import FilterBuilder from './properties/FilterBuilder.svelte';
	import { FIELD_DEFS } from '$lib/entities/meta';
	import { viewsStore } from '$lib/stores/views';
	import {
		propertyFiltersToViewFilter,
		viewFilterToPropertyFilters,
		type View
	} from '$lib/types/view';
	import { treeHasConditions, type FilterTree } from '$lib/types/filter';

	let { type, fileId = null }: { type: EntityType; fileId?: string | null } = $props();

	let items: EntityListItem[] = $state([]);
	let details: Record<string, EntityDetail | null> = $state({});
	let selected: EntityDetail | null = $state(null);
	let loading = $state(true);
	let detailLoading = $state(false);
	let error: string | null = $state(null);
	let creating = $state(false);
	let editingForm = $state(false);
	let editingBody = $state(false);
	let focusMode = $state(false);
	let saveDialogOpen = $state(false);
	let propertyFilters: PropertyFilters = $state(parseFilterParams(page.url.searchParams));
	let compoundFilter: FilterTree | null = $state(null);
	let showFilterBuilder = $state(false);
	let selection: Set<string> = $state(new Set());
	let bulkBusy = $state(false);

	function syncFiltersToUrl(next: PropertyFilters) {
		propertyFilters = next;
		const url = new URL(page.url);
		writeFilterParams(url.searchParams, next);
		void goto(url, { replaceState: true, keepFocus: true, noScroll: true });
	}
	type ViewMode = 'list' | 'table' | 'kanban';
	const view: ViewMode = $derived.by(() => {
		const v = page.url.searchParams.get('view');
		if (v === 'kanban' && type === 'application') return 'kanban';
		if (v === 'table') return 'table';
		return 'list';
	});

	const groupBy = $derived(page.url.searchParams.get('group') === 'application' ? 'application' : 'none');
	const customProperties = $derived($schemaStore.schema?.types[type]?.properties ?? []);
	// Synthetic Property records for the entity type's built-in
	// fields (name, country, city, …) so PropertyFilterBar can show
	// them in the same chip UI alongside custom properties. They're
	// stripped from the backend filter at loadList time and applied
	// in-memory against fetched detail records.
	const builtinFilterableProps = $derived(builtinFilterableProperties(type));
	const filterableProperties = $derived([...customProperties, ...builtinFilterableProps]);

	// True when *anything* is narrowing the list (property filters,
	// tag filters, built-in chips, or compound filter) — drives the
	// empty-state copy.
	const anyFilterActive = $derived(
		hasActiveFilters(propertyFilters) ||
			$filters.tags.length > 0 ||
			treeHasConditions(compoundFilter)
	);

	function clearAllFilters() {
		filters.clear();
		compoundFilter = null;
		const url = new URL(page.url);
		for (const k of ['text', 'num', 'date', 'sort', 'sort_dir']) {
			url.searchParams.delete(k);
		}
		void goto(url, { replaceState: true, keepFocus: true, noScroll: true });
	}

	// Generic group-by used by the list & table views (separate from
	// `kanban_by` which the kanban needs because its "ungrouped" state is
	// status). Use the URL so saved views can capture it later.
	const groupByKey = $derived(page.url.searchParams.get('group') ?? undefined);
	const groupableProps = $derived(
		customProperties.filter((p) => p.type === 'single-select')
	);
	function setGroupByKey(key: string | undefined) {
		const url = new URL(page.url);
		if (!key) url.searchParams.delete('group');
		else url.searchParams.set('group', key);
		void goto(url, { replaceState: true, keepFocus: true, noScroll: true });
	}

	// Single-select properties on Application that the user can group the kanban by.
	const kanbanGroupableProps = $derived(
		type === 'application'
			? customProperties.filter((p) => p.type === 'single-select')
			: []
	);
	const kanbanGroupKey = $derived(page.url.searchParams.get('kanban_by') ?? 'status');
	const kanbanGroupBy = $derived.by(() => {
		if (kanbanGroupKey === 'status') return undefined; // built-in path
		const prop = kanbanGroupableProps.find((p) => p.key === kanbanGroupKey);
		if (!prop || prop.type !== 'single-select') return undefined;
		return {
			key: prop.key,
			columns: prop.options.map((o) => ({ value: o.value, label: o.label, color: o.color }))
		};
	});

	function setKanbanGroup(key: string) {
		const url = new URL(page.url);
		if (key === 'status') url.searchParams.delete('kanban_by');
		else url.searchParams.set('kanban_by', key);
		void goto(url, { replaceState: true, keepFocus: true, noScroll: true });
	}

	const kanbanCardProperties = $derived.by(() => {
		const keys = activeView?.card_properties ?? null;
		if (!keys || keys.length === 0) return [];
		return keys
			.map((k) => customProperties.find((p) => p.key === k))
			.filter((p): p is import('$lib/types/schema').Property => !!p);
	});

	function setView(next: ViewMode) {
		const url = new URL(page.url);
		if (next === 'list') url.searchParams.delete('view');
		else url.searchParams.set('view', next);
		void goto(url, { replaceState: true, keepFocus: true, noScroll: true });
	}

	// Column visibility (table view) — persisted per entity type in
	// localStorage. Defaults to "all custom properties, no built-ins" so the
	// table starts empty-but-functional and the user opts in to the noise.
	const COLUMNS_STORAGE_PREFIX = 'eduport:columns:';

	function loadColumns(t: EntityType): { custom: string[]; builtin: string[] } | null {
		if (typeof window === 'undefined') return null;
		try {
			const raw = window.localStorage.getItem(COLUMNS_STORAGE_PREFIX + t);
			if (!raw) return null;
			const parsed = JSON.parse(raw);
			if (typeof parsed === 'object' && Array.isArray(parsed.custom) && Array.isArray(parsed.builtin)) {
				return { custom: parsed.custom, builtin: parsed.builtin };
			}
		} catch {
			/* ignore */
		}
		return null;
	}

	function saveColumns(t: EntityType, value: { custom: string[]; builtin: string[] }) {
		if (typeof window === 'undefined') return;
		try {
			window.localStorage.setItem(COLUMNS_STORAGE_PREFIX + t, JSON.stringify(value));
		} catch {
			/* localStorage is best-effort */
		}
	}

	let visibleCustomKeys: string[] = $state([]);
	let visibleBuiltinKeys: string[] = $state([]);

	$effect(() => {
		// Whenever entity type changes, restore columns from localStorage,
		// or seed sensible defaults: all custom properties, no built-ins.
		const t = type;
		const stored = loadColumns(t);
		if (stored) {
			visibleCustomKeys = stored.custom;
			visibleBuiltinKeys = stored.builtin;
		} else {
			visibleCustomKeys = customProperties.map((p) => p.key);
			visibleBuiltinKeys = [];
		}
	});

	function persistColumns(next: { custom: string[]; builtin: string[] }) {
		visibleCustomKeys = next.custom;
		visibleBuiltinKeys = next.builtin;
		saveColumns(type, next);
	}

	function setGroupBy(next: 'none' | 'application') {
		const url = new URL(page.url);
		if (next === 'none') url.searchParams.delete('group');
		else url.searchParams.set('group', next);
		void goto(url, { replaceState: true, keepFocus: true, noScroll: true });
	}

	const newAction = getContext<Writable<{ label: string; onClick: () => void } | null>>('eduport:newAction');
	const selectedFileId = $derived(fileId ?? undefined);

	async function loadList() {
		loading = true;
		error = null;
		try {
			// Filter / sort happen on the backend via `Vault::query`
			// (see `eduport_core::query`), which reads any frontmatter
			// field directly. Built-in keys (name, country, city, …)
			// and custom-schema keys are equally first-class — no
			// client-side post-filter, no shadow index split.
			let baseItems: EntityListItem[];
			const treeActive = treeHasConditions(compoundFilter);
			if (hasActiveFilters(propertyFilters) || treeActive) {
				baseItems = await filterEntitiesByProperties(
					type,
					propertyFilters,
					treeActive ? compoundFilter : null
				);
				if ($filters.tags.length > 0) {
					const tagged = new Set(
						(await listEntities(type, $filters.tags)).map((i) => i.file_id)
					);
					baseItems = baseItems.filter((item) => tagged.has(item.file_id));
				}
			} else {
				baseItems = await listEntities(type, $filters.tags);
			}
			const nextDetails: Record<string, EntityDetail | null> = {};
			await Promise.all(
				baseItems.map(async (item) => {
					try {
						nextDetails[item.file_id] = await getEntity(type, item.file_id);
					} catch {
						nextDetails[item.file_id] = null;
					}
				})
			);
			items = baseItems;
			details = nextDetails;
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			loading = false;
		}
	}

	async function loadDetail() {
		if (!fileId) {
			selected = null;
			return;
		}
		detailLoading = true;
		try {
			selected = await getEntity(type, fileId);
			details = { ...details, [fileId]: selected };
		} catch (e) {
			// Stale fileId in the URL — e.g. the user deleted the file
			// externally and then refreshed the app, or the watcher hasn't
			// dropped the row from the index yet. Clear the selection and
			// drop the fileId from the URL so future refreshes don't keep
			// repeating the error; the list still re-renders correctly.
			if (e instanceof CoreCommandError && e.code === 'not_found') {
				selected = null;
				if (page.url.pathname !== `/${type}`) {
					void goto(`/${type}${page.url.search}`, { replaceState: true, keepFocus: true });
				}
			} else {
				error = e instanceof Error ? e.message : String(e);
				selected = null;
			}
		} finally {
			detailLoading = false;
		}
	}

	$effect(() => {
		const _filterKey =
			$filters.tags.join('\u0000') +
			'|' +
			JSON.stringify(propertyFilters) +
			'|' +
			JSON.stringify(compoundFilter);
		newAction?.set({ label: `New ${TYPE_LABELS[type]}`, onClick: () => (creating = true) });
		void loadList();
		return () => newAction?.set(null);
	});

	$effect(() => {
		// Re-parse filters from the URL whenever the entity type or
		// search string actually changes. parseFilterParams returns a
		// fresh object each call, so a naive assignment would cascade
		// into the loadList effect on *any* navigation (including a
		// pathname-only change from /[type] to /[type]/[fileId] when
		// clicking an entity), producing a Loading… flash on every
		// row click. Deep-compare and only reassign on real change.
		type;
		const parsed = parseFilterParams(page.url.searchParams);
		if (JSON.stringify(parsed) !== JSON.stringify(propertyFilters)) {
			propertyFilters = parsed;
		}
	});

	$effect(() => {
		void schemaStore.load();
		void viewsStore.load();
	});

	// Watcher-driven reloads. The workspace is mounted once per
	// /[type]/... layout session (see ../routes/[type]/+layout.svelte);
	// inside it we only refetch when the data actually changed, never
	// on item selection. Sources of "real change":
	//   - external file edits (Obsidian, finder, CLI) → entity_changed/_deleted
	//   - schema or saved-view edits → schema_changed / views_changed
	//   - full rescan requests → needs_rescan
	// In-app writes call `Watcher::note_self_write(path)` before the
	// write, so the watcher suppresses our own events; the explicit
	// loadList/loadDetail calls in onDone handlers below handle those.
	// Events are debounced to coalesce bursts (drag-drop, rapid saves).
	type VaultEventPayload = {
		kind: 'entity_changed' | 'entity_deleted' | 'schema_changed' | 'views_changed' | 'needs_rescan';
		file_id?: string;
		path?: string;
	};

	onMount(() => {
		if (!isTauri()) return;
		let unlisten: (() => void) | null = null;
		let pending: ReturnType<typeof setTimeout> | null = null;
		const schedule = (fn: () => void) => {
			if (pending) clearTimeout(pending);
			pending = setTimeout(() => {
				pending = null;
				fn();
			}, 200);
		};
		void listenCoreEvent<VaultEventPayload>('eduport:vault-event', (payload) => {
			switch (payload.kind) {
				case 'entity_changed':
				case 'entity_deleted':
				case 'needs_rescan':
					schedule(() => {
						void loadList();
						if (payload.file_id && payload.file_id === fileId) {
							void loadDetail();
						}
					});
					break;
				case 'schema_changed':
					void schemaStore.load();
					schedule(() => void loadList());
					break;
				case 'views_changed':
					void viewsStore.load();
					break;
			}
		}).then((u) => {
			unlisten = u;
		});
		return () => {
			if (pending) clearTimeout(pending);
			unlisten?.();
		};
	});

	// Workspace-scoped keyboard shortcuts. Documented in
	// $lib/components/ShortcutsHelp.svelte. These only fire when no
	// modal is open and no text input is focused.
	const anyModalOpen = $derived(
		creating || editingForm || editingBody || saveDialogOpen || focusMode
	);

	function moveSelectionBy(delta: number) {
		if (items.length === 0) return;
		const currentIdx = fileId ? items.findIndex((i) => i.file_id === fileId) : -1;
		let next = currentIdx + delta;
		if (currentIdx === -1) {
			next = delta > 0 ? 0 : items.length - 1;
		}
		next = Math.max(0, Math.min(items.length - 1, next));
		if (next === currentIdx) return;
		const url = new URL(page.url);
		url.pathname = `/${type}/${items[next].file_id}`;
		void goto(url, { keepFocus: true });
	}

	onMount(() => {
		function onKey(event: KeyboardEvent) {
			// Esc exits focus mode first. Other modals own their own
			// Esc handling. We do this before the typing-target check
			// so Esc works even if the user's cursor is in an input.
			if (event.key === 'Escape' && focusMode) {
				event.preventDefault();
				focusMode = false;
				return;
			}
			if (event.key === 'Escape' && selection.size > 0 && !isTypingTarget(event.target)) {
				event.preventDefault();
				clearSelection();
				return;
			}
			if (isTypingTarget(event.target)) return;

			// Detail-panel shortcuts: only with an entity selected. Focus
			// mode still allows these so the user can edit from focus.
			if (selected) {
				if (event.key === 'e' && !event.metaKey && !event.ctrlKey && !event.shiftKey) {
					event.preventDefault();
					editingForm = true;
					return;
				}
				if (event.key === 'E' && event.shiftKey && !event.metaKey && !event.ctrlKey) {
					event.preventDefault();
					editingBody = true;
					return;
				}
				if (event.key === 'f' && !event.metaKey && !event.ctrlKey) {
					event.preventDefault();
					focusMode = !focusMode;
					return;
				}
				if ((event.metaKey || event.ctrlKey) && event.key === 'Backspace') {
					event.preventDefault();
					void handleDelete();
					return;
				}
			}

			// List navigation — suppressed while a modal blocks the list.
			if (anyModalOpen) return;
			if (event.key === 'j' || event.key === 'ArrowDown') {
				event.preventDefault();
				moveSelectionBy(1);
				return;
			}
			if (event.key === 'k' || event.key === 'ArrowUp') {
				event.preventDefault();
				moveSelectionBy(-1);
				return;
			}
			if (event.key === 'g' && !event.shiftKey) {
				event.preventDefault();
				if (items.length > 0) {
					const url = new URL(page.url);
					url.pathname = `/${type}/${items[0].file_id}`;
					void goto(url, { keepFocus: true });
				}
				return;
			}
			if (event.key === 'G' && event.shiftKey) {
				event.preventDefault();
				if (items.length > 0) {
					const url = new URL(page.url);
					url.pathname = `/${type}/${items[items.length - 1].file_id}`;
					void goto(url, { keepFocus: true });
				}
				return;
			}
		}
		window.addEventListener('keydown', onKey);
		return () => window.removeEventListener('keydown', onKey);
	});

	// Active view tracking. URL param `?view_id=<id>` selects a saved view;
	// when active, edits to filter/sort/group propagate into the URL but the
	// view tab stays highlighted (state can diverge — saving updates the view).
	const activeViewId = $derived(page.url.searchParams.get('view_id'));
	const activeView = $derived.by((): View | null => {
		if (!activeViewId) return null;
		return $viewsStore.file?.types[type]?.views.find((v) => v.id === activeViewId) ?? null;
	});

	function applyView(view: View | null) {
		const url = new URL(page.url);
		// Clear all view-driven URL params first.
		for (const k of ['text', 'num', 'date', 'sort', 'sort_dir', 'group', 'view', 'view_id', 'kanban_by']) {
			url.searchParams.delete(k);
		}
		// Compound filter is in-memory state, not URL — apply (or
		// clear) it whenever a saved view is loaded.
		compoundFilter = view?.filter_tree ?? null;
		if (view) {
			url.searchParams.set('view_id', view.id);
			const pf = viewFilterToPropertyFilters(view);
			writeFilterParams(url.searchParams, pf);
			if (view.group_by_key) url.searchParams.set('group', view.group_by_key);
			if (view.kind === 'table') url.searchParams.set('view', 'table');
			else if (view.kind === 'board' && type === 'application') url.searchParams.set('view', 'kanban');
			// list is the default — no param needed
			// Apply view's columns to localStorage so the table picks them up.
			if (view.columns) {
				visibleCustomKeys = view.columns;
				saveColumns(type, { custom: view.columns, builtin: visibleBuiltinKeys });
			}
		}
		void goto(url, { replaceState: false, keepFocus: false });
	}

	function captureCurrentAsViewBody() {
		return {
			viewKind: (view === 'kanban' ? 'board' : view) as 'list' | 'table' | 'board',
			filter: propertyFiltersToViewFilter(propertyFilters),
			filterTree: compoundFilter ?? null,
			sortKey: propertyFilters.sort ?? null,
			sortDir: propertyFilters.sortDir ?? 'asc',
			groupByKey: groupByKey ?? null,
			columns: visibleCustomKeys.length > 0 ? visibleCustomKeys : null,
			cardProperties: null
		} as const;
	}

	// "Save changes to view" — overwrite the currently active view's
	// stored state with whatever filter / sort / group / columns / view
	// kind the user has set right now. Views are otherwise immutable
	// snapshots: clicking the tab restores the snapshot exactly, so
	// any unsaved tweaks are lost on the next tab switch.
	async function updateActiveView() {
		const v = activeView;
		if (!v) return;
		const body = captureCurrentAsViewBody();
		try {
			await viewsStore.update(type, v.id, {
				name: v.name,
				kind: body.viewKind,
				filter: body.filter,
				filter_tree: body.filterTree,
				sort_key: body.sortKey,
				sort_dir: body.sortDir,
				group_by_key: body.groupByKey,
				columns: body.columns,
				card_properties: body.cardProperties
			});
		} catch (e) {
			toasts.error(
				"Couldn't save changes to view",
				e instanceof Error ? e.message : String(e)
			);
		}
	}

	$effect(() => {
		void loadDetail();
	});

	async function handleDelete() {
		if (!selected || !fileId) return;
		const name = selected.entity.name as string;
		if (($settings?.confirm_deletes ?? true) && !(await confirmDestructive(`Move "${name}" to trash?`))) return;
		try {
			await deleteEntity(type, fileId);
			selected = null;
			focusMode = false;
			await loadList();
			goto(`/${type}`);
			toasts.success(`Moved "${name}" to trash`);
		} catch (e) {
			toasts.error('Delete failed', e instanceof Error ? e.message : String(e));
		}
	}

	async function bulkDelete() {
		if (selection.size === 0) return;
		const n = selection.size;
		if (
			($settings?.confirm_deletes ?? true) &&
			!(await confirmDestructive(`Move ${n} ${type}${n === 1 ? '' : 's'} to trash?`))
		) {
			return;
		}
		bulkBusy = true;
		const ids = Array.from(selection);
		const results = await Promise.allSettled(ids.map((id) => deleteEntity(type, id)));
		const failed = results.filter((r) => r.status === 'rejected').length;
		bulkBusy = false;
		selection = new Set();
		await loadList();
		if (failed === 0) toasts.success(`Moved ${n} item${n === 1 ? '' : 's'} to trash`);
		else toasts.error(`${failed}/${n} delete${n === 1 ? '' : 's'} failed`);
	}

	function clearSelection() {
		selection = new Set();
	}

	// Right-click context menu on list rows. Renders fixed at the
	// click coordinates so it never gets clipped by the section's
	// overflow:hidden — same approach used for the three-dot menus
	// in DetailPanel and ViewTabs.
	let ctxMenu:
		| { x: number; y: number; item: EntityListItem }
		| null = $state(null);

	function openContextMenu(event: MouseEvent, item: EntityListItem) {
		// Keep the menu inside the viewport on the right and bottom
		// edges. 200x180 is a generous bounding box for our menu items.
		const x = Math.min(event.clientX, window.innerWidth - 200);
		const y = Math.min(event.clientY, window.innerHeight - 180);
		ctxMenu = { x, y, item };
	}

	$effect(() => {
		if (!ctxMenu) return;
		function close() {
			ctxMenu = null;
		}
		function onKey(e: KeyboardEvent) {
			if (e.key === 'Escape') close();
		}
		const id = window.setTimeout(() => {
			window.addEventListener('click', close);
		}, 0);
		window.addEventListener('keydown', onKey);
		window.addEventListener('scroll', close, true);
		return () => {
			window.clearTimeout(id);
			window.removeEventListener('click', close);
			window.removeEventListener('keydown', onKey);
			window.removeEventListener('scroll', close, true);
		};
	});

	function entityPathFor(fileIdValue: string): string | null {
		if (!$settings) return null;
		return `${$settings.data_folder.replace(/\/$/, '')}/${fileIdValue}.md`;
	}

	async function ctxOpen() {
		if (!ctxMenu) return;
		const id = ctxMenu.item.file_id;
		ctxMenu = null;
		const url = new URL(page.url);
		url.pathname = `/${type}/${id}`;
		await goto(url, { keepFocus: true });
	}

	async function ctxReveal() {
		if (!ctxMenu) return;
		const path = entityPathFor(ctxMenu.item.file_id);
		ctxMenu = null;
		if (!path) return;
		try {
			await revealInFileManager(path);
		} catch (e) {
			toasts.error('Reveal failed', e instanceof Error ? e.message : String(e));
		}
	}

	async function ctxCopyId() {
		if (!ctxMenu) return;
		const id = ctxMenu.item.file_id;
		ctxMenu = null;
		try {
			await navigator.clipboard.writeText(id);
			toasts.success('Copied file ID', id);
		} catch {
			toasts.error('Clipboard unavailable');
		}
	}

	async function ctxDelete() {
		if (!ctxMenu) return;
		const item = ctxMenu.item;
		ctxMenu = null;
		if (
			($settings?.confirm_deletes ?? true) &&
			!(await confirmDestructive(`Move "${item.name}" to trash?`))
		) {
			return;
		}
		try {
			await deleteEntity(type, item.file_id);
			if (fileId === item.file_id) {
				selected = null;
				focusMode = false;
				goto(`/${type}`);
			}
			await loadList();
			toasts.success(`Moved "${item.name}" to trash`);
		} catch (e) {
			toasts.error('Delete failed', e instanceof Error ? e.message : String(e));
		}
	}
</script>

<div class="grid h-full min-h-0 grid-cols-[minmax(360px,1fr)_minmax(360px,440px)] overflow-hidden">
	<section class="flex min-w-0 flex-col overflow-hidden border-r border-[var(--color-border)]">
		<ViewTabs
			entityType={type}
			activeViewId={activeViewId ?? null}
			onSelect={applyView}
			onSaveCurrent={() => (saveDialogOpen = true)}
			onUpdateActive={updateActiveView}
			onActiveDeleted={() => applyView(null)}
		/>
		<header class="flex items-center justify-between border-b border-[var(--color-border)] px-4 py-2">
			<div>
				<h1 class="text-sm font-semibold">{TYPE_LABELS[type]}</h1>
				<p class="text-[10px] uppercase tracking-wider text-[var(--color-muted)]">{items.length} items</p>
			</div>
			<div class="flex items-center gap-2">
				{#if view === 'table'}
					<ColumnVisibilityMenu
						properties={customProperties}
						builtinFields={FIELD_DEFS[type] ?? []}
						{visibleCustomKeys}
						{visibleBuiltinKeys}
						onChange={persistColumns}
					/>
				{/if}
				{#if (view === 'list' || view === 'table') && groupableProps.length > 0}
					<label class="flex items-center gap-1 text-xs">
						<span class="text-[var(--color-muted)]">Group by</span>
						<select
							value={groupByKey ?? ''}
							onchange={(e) => setGroupByKey((e.currentTarget as HTMLSelectElement).value || undefined)}
							class="rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-2 py-0.5 text-xs"
						>
							<option value="">(none)</option>
							{#each groupableProps as p}
								<option value={p.key}>{p.name}</option>
							{/each}
						</select>
					</label>
				{/if}
				{#if type === 'application' && view === 'kanban' && kanbanGroupableProps.length > 0}
					<label class="flex items-center gap-1 text-xs">
						<span class="text-[var(--color-muted)]">Group by</span>
						<select
							value={kanbanGroupKey}
							onchange={(e) => setKanbanGroup((e.currentTarget as HTMLSelectElement).value)}
							class="rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-2 py-0.5 text-xs"
						>
							<option value="status">Status (built-in)</option>
							{#each kanbanGroupableProps as p}
								<option value={p.key}>{p.name}</option>
							{/each}
						</select>
					</label>
				{/if}
				{#if type === 'application' && view === 'kanban' && activeView}
					<CardPropertiesMenu
						entityType={type}
						properties={customProperties}
						{activeView}
					/>
				{/if}
				{#if type === 'email'}
					<div class="flex rounded border border-[var(--color-border)] p-0.5 text-xs">
						<button class="rounded px-2 py-1" class:active={groupBy === 'none'} onclick={() => setGroupBy('none')}>Chronological</button>
						<button class="rounded px-2 py-1" class:active={groupBy === 'application'} onclick={() => setGroupBy('application')}>By app</button>
					</div>
				{/if}
				<div class="flex rounded border border-[var(--color-border)] p-0.5 text-xs">
					<button class="rounded px-2 py-1" class:active={view === 'list'} onclick={() => setView('list')}>List</button>
					<button class="rounded px-2 py-1" class:active={view === 'table'} onclick={() => setView('table')}>Table</button>
					{#if type === 'application'}
						<button class="rounded px-2 py-1" class:active={view === 'kanban'} onclick={() => setView('kanban')}>Kanban</button>
					{/if}
				</div>
			</div>
		</header>

		{#if filterableProperties.length > 0 && !(type === 'application' && view === 'kanban')}
			<PropertyFilterBar
				properties={filterableProperties}
				filters={propertyFilters}
				onChange={syncFiltersToUrl}
			/>
			<div class="border-b border-[var(--color-border)] px-4 py-2">
				<button
					type="button"
					class="text-xs text-[var(--color-muted)] hover:text-[var(--color-text)]"
					onclick={() => (showFilterBuilder = !showFilterBuilder)}
				>
					{showFilterBuilder ? '▾' : '▸'} Compound filter{treeHasConditions(compoundFilter)
						? ` (active)`
						: ''}
				</button>
				{#if showFilterBuilder}
					<div class="mt-2">
						<FilterBuilder
							tree={compoundFilter}
							properties={filterableProperties}
							onChange={(next) => (compoundFilter = next)}
						/>
					</div>
				{/if}
			</div>
		{/if}

		{#if selection.size > 0}
			<div
				class="flex items-center gap-3 border-b border-[var(--color-border)] bg-[var(--color-accent)]/10 px-4 py-2 text-xs"
			>
				<span class="font-medium">
					{selection.size} selected
				</span>
				<button
					class="rounded border border-[var(--color-border)] bg-white/5 px-2 py-1 hover:bg-white/10 disabled:opacity-50"
					onclick={bulkDelete}
					disabled={bulkBusy}
				>
					{bulkBusy ? 'Deleting…' : `Delete ${selection.size}`}
				</button>
				<button
					class="rounded border border-[var(--color-border)] bg-white/5 px-2 py-1 hover:bg-white/10"
					onclick={clearSelection}
				>
					Clear (Esc)
				</button>
			</div>
		{/if}

		<div class="min-h-0 flex-1 overflow-auto">
			{#if loading}
				<EntityListSkeleton />
			{:else if error}
				<div class="p-8 text-center text-[var(--color-bad)]">Error: {error}</div>
			{:else if type === 'application' && view === 'kanban'}
				<KanbanBoard
					groupBy={kanbanGroupBy}
					cardProperties={kanbanCardProperties}
					onPick={(id) => {
						const url = new URL(page.url);
						url.pathname = `/application/${id}`;
						void goto(url, { keepFocus: true });
					}}
					onUpdated={(id) => {
						void loadList();
						if (id === selectedFileId) void loadDetail();
					}}
				/>
			{:else if view === 'table'}
				<TableView
					entityType={type}
					{items}
					{details}
					properties={customProperties}
					{visibleCustomKeys}
					{visibleBuiltinKeys}
					selectedFileId={selectedFileId}
					sortKey={propertyFilters.sort}
					sortDir={propertyFilters.sortDir}
					{groupByKey}
					onSort={(key, dir) => {
						syncFiltersToUrl({ ...propertyFilters, sort: key, sortDir: key ? dir : undefined });
					}}
					onUpdated={(id) => {
						void loadList();
						if (id === selectedFileId) void loadDetail();
					}}
				/>
			{:else if type === 'email' && groupBy === 'application'}
				<EmailGroupedList {items} />
			{:else if groupByKey}
				{@const groupProp = groupableProps.find((p) => p.key === groupByKey) ?? null}
				<GroupedList
					entityType={type}
					{items}
					{details}
					groupBy={groupProp}
					{selectedFileId}
					onContextMenu={openContextMenu}
				/>
			{:else}
				<EntityList
					{items}
					{type}
					{selectedFileId}
					{details}
					{selection}
					onSelectionChange={(next) => (selection = next)}
					onContextMenu={openContextMenu}
					filtersActive={anyFilterActive}
					onClearFilters={clearAllFilters}
				/>
			{/if}
		</div>
	</section>

	<aside class="min-w-0 overflow-hidden bg-[var(--color-panel)]">
		{#if detailLoading}
			<div class="p-8 text-center text-[var(--color-muted)]">Loading detail...</div>
		{:else if selected}
			<DetailPanel
				detail={selected}
				onEditForm={() => (editingForm = true)}
				onEditBody={() => (editingBody = true)}
				onFocus={() => (focusMode = true)}
				onDelete={handleDelete}
			/>
		{:else}
			<div class="flex h-full items-center justify-center p-8 text-center text-sm text-[var(--color-muted)]">
				Select an item to inspect fields, body, backlinks, and actions.
			</div>
		{/if}
	</aside>
</div>

{#if focusMode && selected}
	<div class="fixed inset-0 z-40 flex items-center justify-center bg-black/70 p-8">
		<div class="h-[88vh] w-[min(1120px,92vw)] overflow-hidden rounded-lg border border-[var(--color-border)] bg-[var(--color-panel)] shadow-2xl">
			<DetailPanel
				detail={selected}
				focusMode={true}
				onEditForm={() => (editingForm = true)}
				onEditBody={() => (editingBody = true)}
				onFocus={() => (focusMode = false)}
				onDelete={handleDelete}
			/>
		</div>
	</div>
{/if}

{#if creating}
	<EntityForm
		{type}
		onCancel={() => (creating = false)}
		onDone={(id) => {
			creating = false;
			void loadList();
			goto(`/${type}/${id}`);
		}}
	/>
{/if}

{#if editingForm && selected && fileId}
	<EntityForm
		{type}
		{fileId}
		includeBody={false}
		initial={{ frontmatter: selected.entity, body: selected.body }}
		onCancel={() => (editingForm = false)}
		onDone={() => {
			editingForm = false;
			void loadList();
			void loadDetail();
		}}
	/>
{/if}

{#if editingBody && selected}
	<EntityBodyEditor
		detail={selected}
		onCancel={() => (editingBody = false)}
		onDone={() => {
			editingBody = false;
			void loadList();
			void loadDetail();
		}}
	/>
{/if}

{#if saveDialogOpen}
	{@const body = captureCurrentAsViewBody()}
	<SaveViewDialog
		entityType={type}
		mode="create"
		viewKind={body.viewKind}
		filter={body.filter}
		filterTree={body.filterTree}
		sortKey={body.sortKey}
		sortDir={body.sortDir}
		groupByKey={body.groupByKey}
		columns={body.columns}
		cardProperties={body.cardProperties}
		onCancel={() => (saveDialogOpen = false)}
		onSaved={(view) => {
			saveDialogOpen = false;
			// Activate the new view so the user sees its tab highlighted.
			const url = new URL(page.url);
			url.searchParams.set('view_id', view.id);
			void goto(url, { replaceState: true });
		}}
	/>
{/if}

{#if ctxMenu}
	<!-- Right-click context menu for list rows. Same fixed-position
		 pattern as the three-dot menus so it can't be clipped by the
		 nested overflow contexts. Click-outside / Escape / scroll all
		 dismiss via the $effect above. -->
	<div
		role="menu"
		class="fixed z-50 w-48 overflow-hidden rounded border border-[var(--color-border)] bg-[var(--color-panel)] text-xs shadow-xl"
		style="top: {ctxMenu.y}px; left: {ctxMenu.x}px"
	>
		<button class="block w-full px-3 py-2 text-left hover:bg-white/5" onclick={ctxOpen}>
			Open
		</button>
		<button class="block w-full px-3 py-2 text-left hover:bg-white/5" onclick={ctxReveal}>
			Show in file manager
		</button>
		<button class="block w-full px-3 py-2 text-left hover:bg-white/5" onclick={ctxCopyId}>
			Copy file ID
		</button>
		<button
			class="block w-full border-t border-[var(--color-border)] px-3 py-2 text-left text-[var(--color-bad)] hover:bg-red-900/30"
			onclick={ctxDelete}
		>
			Move to trash
		</button>
	</div>
{/if}

<style>
	.active {
		background-color: rgba(108, 182, 255, 0.15);
		color: var(--color-accent);
	}
</style>
