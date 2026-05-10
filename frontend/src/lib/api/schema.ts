/**
 * Schema editor + property aggregation API client.
 *
 * Phase 10 cutover: Tauri command channel via `coreInvoke`.
 * Filter parsing helpers (`buildFilterQuery`, `parseFilterParams`,
 * `writeFilterParams`, `hasActiveFilters`) stay because the URL
 * is the source of truth for view-state — they don't reach the
 * backend.
 */

import { coreInvoke } from './client';
import type { EntityType, EntityListItem } from '../types';
import type {
	EntityTypeSchema,
	FullSchema,
	Property,
	PropertyCount,
	PropertyFilters,
	SelectOption
} from '../types/schema';

// ----- schema CRUD ----------------------------------------------------------

export function getSchema(): Promise<FullSchema> {
	return coreInvoke('core_schema_get');
}

export function getTypeSchema(type: EntityType): Promise<EntityTypeSchema> {
	return coreInvoke('core_schema_get_type', { entityType: type });
}

export function addProperty(type: EntityType, prop: Property): Promise<EntityTypeSchema> {
	return coreInvoke('core_schema_add_property', { entityType: type, property: prop });
}

export interface PropertyPatch {
	name?: string;
	description?: string | null;
	required?: boolean;
	default?: unknown;
	unit?: string | null;
	options?: SelectOption[];
	target_types?: EntityType[];
}

export function patchProperty(
	type: EntityType,
	key: string,
	patch: PropertyPatch
): Promise<EntityTypeSchema> {
	return coreInvoke('core_schema_patch_property', { entityType: type, key, patch });
}

export function deleteProperty(type: EntityType, key: string): Promise<EntityTypeSchema> {
	return coreInvoke('core_schema_delete_property', { entityType: type, key });
}

export function reorderProperties(
	type: EntityType,
	orderedKeys: string[]
): Promise<EntityTypeSchema> {
	return coreInvoke('core_schema_reorder_properties', {
		entityType: type,
		orderedKeys
	});
}

export function applyTierTemplate(types: EntityType[]): Promise<{
	results: Record<string, { status: 'added' | 'exists' }>;
	schema: FullSchema;
}> {
	return coreInvoke('core_schema_apply_tier_template', { types });
}

export function purgeOrphans(
	type: EntityType,
	key: string
): Promise<{ rewritten: number; skipped: { file_id: string; reason: string }[] }> {
	return coreInvoke('core_schema_purge_orphans', { entityType: type, key });
}

// ----- property aggregations / filtering ------------------------------------

export function getPropertyCounts(
	type: EntityType,
	key: string
): Promise<{ entity_type: EntityType; key: string; values: PropertyCount[] }> {
	return coreInvoke('core_property_value_counts', { entityType: type, key });
}

export function filterEntitiesByProperties(
	type: EntityType,
	filters: PropertyFilters
): Promise<EntityListItem[]> {
	// Range tuples (`[lo, hi]`) deserialize directly into Rust
	// `(Option<f64>, Option<f64>)` etc. — forwarded verbatim. Empty
	// text values, however, mean "(any)" in the chip UI and must be
	// stripped here: the index treats `value_text = ''` as a literal
	// match and would return zero rows for a tier chip set to (any).
	return coreInvoke('core_filter_entities_by_properties', {
		entityType: type,
		filters: {
			text: Object.fromEntries(
				Object.entries(filters.text).filter(([, v]) => v !== '')
			),
			num: filters.num,
			date: filters.date,
			sort: filters.sort ?? null,
			sort_dir: filters.sortDir ?? null
		}
	});
}

/** Returns true when at least one filter / sort actively narrows the
 * list. Chips with an empty value (single-select set to "(any)") don't
 * count — they're a configured-but-unconstrained placeholder. */
export function hasActiveFilters(filters: PropertyFilters): boolean {
	return (
		Object.values(filters.text).some((v) => v !== '') ||
		Object.keys(filters.num).length > 0 ||
		Object.keys(filters.date).length > 0 ||
		!!filters.sort
	);
}

// ----- URL-shaped filter helpers (unchanged) --------------------------------
//
// These helpers run entirely in the frontend — they decode and
// encode `PropertyFilters` against `URLSearchParams` so the
// browser URL captures view state. They never reach the backend
// and are unaffected by the transport swap.

function encodeRange(lo: number | string | null, hi: number | string | null): string {
	const left = lo === null || lo === undefined ? '' : String(lo);
	const right = hi === null || hi === undefined ? '' : String(hi);
	return `${left}..${right}`;
}

export function buildFilterQuery(filters: PropertyFilters): string {
	const params = new URLSearchParams();
	for (const [key, value] of Object.entries(filters.text)) {
		params.append('text', `${key}:${value}`);
	}
	for (const [key, [lo, hi]] of Object.entries(filters.num)) {
		params.append('num', `${key}:${encodeRange(lo, hi)}`);
	}
	for (const [key, [lo, hi]] of Object.entries(filters.date)) {
		params.append('date', `${key}:${encodeRange(lo, hi)}`);
	}
	if (filters.sort) {
		params.append('sort', filters.sort);
		if (filters.sortDir) params.append('sort_dir', filters.sortDir);
	}
	const qs = params.toString();
	return qs ? `?${qs}` : '';
}

export function parseFilterParams(params: URLSearchParams): PropertyFilters {
	const out: PropertyFilters = { text: {}, num: {}, date: {} };
	for (const v of params.getAll('text')) {
		const idx = v.indexOf(':');
		if (idx === -1) continue;
		out.text[v.slice(0, idx)] = v.slice(idx + 1);
	}
	const splitRange = (s: string): [string | null, string | null] => {
		if (!s.includes('..')) return [s, s];
		const [lo, hi] = s.split('..');
		return [lo || null, hi || null];
	};
	for (const v of params.getAll('num')) {
		const idx = v.indexOf(':');
		if (idx === -1) continue;
		const [lo, hi] = splitRange(v.slice(idx + 1));
		out.num[v.slice(0, idx)] = [lo === null ? null : Number(lo), hi === null ? null : Number(hi)];
	}
	for (const v of params.getAll('date')) {
		const idx = v.indexOf(':');
		if (idx === -1) continue;
		const [lo, hi] = splitRange(v.slice(idx + 1));
		out.date[v.slice(0, idx)] = [lo, hi];
	}
	const sort = params.get('sort');
	if (sort) {
		out.sort = sort;
		const dir = params.get('sort_dir');
		out.sortDir = dir === 'desc' ? 'desc' : 'asc';
	}
	return out;
}

export function writeFilterParams(params: URLSearchParams, filters: PropertyFilters): void {
	for (const k of ['text', 'num', 'date', 'sort', 'sort_dir']) params.delete(k);
	for (const [key, value] of Object.entries(filters.text)) {
		params.append('text', `${key}:${value}`);
	}
	const enc = (lo: number | string | null, hi: number | string | null) =>
		`${lo === null ? '' : String(lo)}..${hi === null ? '' : String(hi)}`;
	for (const [key, [lo, hi]] of Object.entries(filters.num)) {
		params.append('num', `${key}:${enc(lo, hi)}`);
	}
	for (const [key, [lo, hi]] of Object.entries(filters.date)) {
		params.append('date', `${key}:${enc(lo, hi)}`);
	}
	if (filters.sort) {
		params.set('sort', filters.sort);
		if (filters.sortDir) params.set('sort_dir', filters.sortDir);
	}
}
