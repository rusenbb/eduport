/**
 * Saved-view types — mirror sidecar's models/view.py.
 */

import type { EntityType } from '../types';
import type { PropertyFilters } from './schema';

export type ViewKind = 'list' | 'table' | 'board';
export type SortDir = 'asc' | 'desc';

export interface ViewFilter {
	text: Record<string, string>;
	num: Record<string, [number | null, number | null]>;
	date: Record<string, [string | null, string | null]>;
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

export interface TypeViews {
	entity_type: EntityType;
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

/** Convert ViewFilter (sidecar shape) to PropertyFilters (frontend). */
export function viewFilterToPropertyFilters(view: View): PropertyFilters {
	return {
		text: { ...view.filter.text },
		num: Object.fromEntries(
			Object.entries(view.filter.num).map(([k, [lo, hi]]) => [k, [lo, hi]])
		),
		date: Object.fromEntries(
			Object.entries(view.filter.date).map(([k, [lo, hi]]) => [k, [lo, hi]])
		),
		sort: view.sort_key ?? undefined,
		sortDir: view.sort_dir
	};
}
