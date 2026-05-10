/**
 * Saved-view types — mirror sidecar's models/view.py.
 */

import type { EntityType } from '../types';
import type { PropertyFilters } from './schema';

export type ViewKind = 'list' | 'table' | 'board';
export type SortDir = 'asc' | 'desc';

// The Rust side serializes empty maps as omitted (skip_serializing_if),
// so any of these three may be missing on the wire. Mark optional and
// guard at the boundary (see viewFilterToPropertyFilters).
export interface ViewFilter {
	text?: Record<string, string>;
	num?: Record<string, [number | null, number | null]>;
	date?: Record<string, [string | null, string | null]>;
}

export interface View {
	id: string;
	name: string;
	kind: ViewKind;
	filter: ViewFilter;
	sort_key?: string | null;
	sort_dir: SortDir;
	group_by_key?: string | null;
	columns?: string[] | null;
	card_properties?: string[] | null;
}

// `entity_type` is *not* in the Rust serialization (it only carries
// `views`). The map key in `ViewsFile.types` is the entity type; the
// caller already knows which type they're working with.
export interface TypeViews {
	views: View[];
}

export interface ViewsFile {
	version: number;
	types: Record<EntityType, TypeViews>;
}

/** Convert PropertyFilters (frontend) to ViewFilter (sidecar shape). */
export function propertyFiltersToViewFilter(filters: PropertyFilters): ViewFilter {
	return {
		text: { ...filters.text },
		num: Object.fromEntries(
			Object.entries(filters.num).map(([k, [lo, hi]]) => [k, [lo, hi]])
		),
		date: Object.fromEntries(
			Object.entries(filters.date).map(([k, [lo, hi]]) => [k, [lo, hi]])
		)
	};
}

/** Convert ViewFilter (backend shape) to PropertyFilters (frontend).
 *
 * The Rust `ViewFilter` struct uses `#[serde(skip_serializing_if = "BTreeMap::is_empty")]`
 * on each of `text` / `num` / `date`, so when a view has no filters (or
 * just one of the three groups is populated) the omitted fields come
 * back as `undefined`. Without the `?? {}` defaults, the second click
 * on a saved-view tab threw `Object.entries(undefined)` and applyView
 * silently aborted before the navigation — which is why "views didn't
 * work" after the first save.
 */
export function viewFilterToPropertyFilters(view: View): PropertyFilters {
	const filter = view.filter ?? {};
	return {
		text: { ...(filter.text ?? {}) },
		num: Object.fromEntries(
			Object.entries(filter.num ?? {}).map(([k, [lo, hi]]) => [k, [lo, hi]])
		),
		date: Object.fromEntries(
			Object.entries(filter.date ?? {}).map(([k, [lo, hi]]) => [k, [lo, hi]])
		),
		sort: view.sort_key ?? undefined,
		sortDir: view.sort_dir
	};
}
