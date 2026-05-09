/**
 * Schema editor + property aggregation API client.
 *
 * Endpoints:
 *   GET    /api/schema
 *   GET    /api/schema/types/{type}
 *   POST   /api/schema/types/{type}/properties
 *   PATCH  /api/schema/types/{type}/properties/{key}
 *   DELETE /api/schema/types/{type}/properties/{key}
 *   POST   /api/schema/templates/tier
 *   POST   /api/schema/types/{type}/properties/{key}/purge_orphans
 *
 *   GET    /api/properties/counts/{type}/{key}
 *   GET    /api/properties/filter/{type}?text=...&num=...&date=...&sort=...&sort_dir=...
 */

import { apiFetch } from './client';
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
	return apiFetch('/api/schema');
}

export function getTypeSchema(type: EntityType): Promise<EntityTypeSchema> {
	return apiFetch(`/api/schema/types/${type}`);
}

export function addProperty(type: EntityType, prop: Property): Promise<EntityTypeSchema> {
	return apiFetch(`/api/schema/types/${type}/properties`, {
		method: 'POST',
		body: JSON.stringify(prop)
	});
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
	return apiFetch(`/api/schema/types/${type}/properties/${encodeURIComponent(key)}`, {
		method: 'PATCH',
		body: JSON.stringify(patch)
	});
}

export function deleteProperty(type: EntityType, key: string): Promise<EntityTypeSchema> {
	return apiFetch(`/api/schema/types/${type}/properties/${encodeURIComponent(key)}`, {
		method: 'DELETE'
	});
}

export function reorderProperties(
	type: EntityType,
	orderedKeys: string[]
): Promise<EntityTypeSchema> {
	return apiFetch(`/api/schema/types/${type}/reorder`, {
		method: 'POST',
		body: JSON.stringify({ ordered_keys: orderedKeys })
	});
}

export function applyTierTemplate(types: EntityType[]): Promise<{
	results: Record<string, { status: 'added' | 'exists' }>;
	schema: FullSchema;
}> {
	return apiFetch('/api/schema/templates/tier', {
		method: 'POST',
		body: JSON.stringify({ types })
	});
}

export function purgeOrphans(
	type: EntityType,
	key: string
): Promise<{ rewritten: number; skipped: { file_id: string; reason: string }[] }> {
	return apiFetch(
		`/api/schema/types/${type}/properties/${encodeURIComponent(key)}/purge_orphans`,
		{ method: 'POST' }
	);
}

// ----- property aggregations / filtering ------------------------------------

export function getPropertyCounts(
	type: EntityType,
	key: string
): Promise<{ entity_type: EntityType; key: string; values: PropertyCount[] }> {
	return apiFetch(`/api/properties/counts/${type}/${encodeURIComponent(key)}`);
}

function encodeRange(lo: number | string | null, hi: number | string | null): string {
	const left = lo === null || lo === undefined ? '' : String(lo);
	const right = hi === null || hi === undefined ? '' : String(hi);
	return `${left}..${right}`;
}

/** Serialize PropertyFilters into the query string that /api/properties/filter expects. */
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

export function filterEntitiesByProperties(
	type: EntityType,
	filters: PropertyFilters
): Promise<EntityListItem[]> {
	return apiFetch(`/api/properties/filter/${type}${buildFilterQuery(filters)}`);
}

/** Returns true when at least one filter / sort is active. */
export function hasActiveFilters(filters: PropertyFilters): boolean {
	return (
		Object.keys(filters.text).length > 0 ||
		Object.keys(filters.num).length > 0 ||
		Object.keys(filters.date).length > 0 ||
		!!filters.sort
	);
}

/** Decode PropertyFilters from URLSearchParams. Mirrors `buildFilterQuery`. */
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

/** Apply filter parameters onto a URLSearchParams, removing any pre-existing
 * `text` / `num` / `date` / `sort` / `sort_dir` entries. */
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
