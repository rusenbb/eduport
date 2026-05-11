/**
 * Centralized view pipeline (Phase C).
 *
 * Group-key extraction for every supported property type — pulled
 * out of the per-component grouping shims (GroupedList, TableView,
 * KanbanBoard) so adding a new grouping type is a single change.
 *
 * The Notion model is: pick any property, get a buckets list back.
 * Bucket *labels* and *colours* differ per type:
 *
 * - single-select: one bucket per option (label/colour from schema)
 * - multi-select:  same — but an item can appear in multiple buckets
 * - date:          buckets keyed by day / week / month / year
 * - number:        buckets keyed by integer step ranges
 * - text / url:    buckets keyed by first letter (uppercase)
 *
 * Items whose value doesn't fit any bucket land in the
 * `__uncategorized__` group. Empty groups are still emitted for
 * select-typed properties (the UI keeps them visible so the user
 * can drag onto an empty column on a board).
 */

import type { EntityListItem } from '$lib/types';
import type {
	Property,
	SelectOption,
	SingleSelectProperty,
	MultiSelectProperty
} from '$lib/types/schema';

export type GroupGranularity = 'day' | 'week' | 'month' | 'year';

export interface GroupSpec {
	property: Property;
	/** Date-bucket granularity. Only meaningful for date properties. */
	dateGranularity?: GroupGranularity;
	/** Number-bucket step. Only meaningful for number properties. */
	numberStep?: number;
}

export interface Bucket {
	value: string;
	label: string;
	color: string;
	items: EntityListItem[];
}

const UNCATEGORIZED: { value: string; label: string; color: string } = {
	value: '__uncategorized__',
	label: 'Uncategorized',
	color: 'gray'
};

function readField(
	details: Record<string, { entity: Record<string, unknown> } | null>,
	item: EntityListItem,
	key: string
): unknown {
	const e = details[item.file_id]?.entity;
	return e?.[key];
}

function ensure(
	map: Map<string, Bucket>,
	value: string,
	label: string,
	color: string
): Bucket {
	let b = map.get(value);
	if (!b) {
		b = { value, label, color, items: [] };
		map.set(value, b);
	}
	return b;
}

function fromSelect(prop: SingleSelectProperty | MultiSelectProperty): Map<string, Bucket> {
	const m = new Map<string, Bucket>();
	for (const o of prop.options) {
		m.set(o.value, { value: o.value, label: o.label, color: o.color, items: [] });
	}
	return m;
}

function dateBucket(iso: string, granularity: GroupGranularity): { value: string; label: string } | null {
	// Accept YYYY-MM-DD; degrade gracefully on anything else.
	if (!/^\d{4}-\d{2}-\d{2}$/.test(iso)) return null;
	const [y, m, d] = iso.split('-');
	switch (granularity) {
		case 'day':
			return { value: iso, label: iso };
		case 'month':
			return { value: `${y}-${m}`, label: `${y}-${m}` };
		case 'year':
			return { value: y, label: y };
		case 'week': {
			// ISO week — Mon=1. Pick the Monday of that week as the bucket label.
			const dt = new Date(`${iso}T00:00:00Z`);
			const day = dt.getUTCDay() || 7; // Sun=7
			dt.setUTCDate(dt.getUTCDate() - (day - 1));
			const wy = dt.getUTCFullYear();
			const wm = String(dt.getUTCMonth() + 1).padStart(2, '0');
			const wd = String(dt.getUTCDate()).padStart(2, '0');
			return { value: `${wy}-${wm}-${wd}`, label: `Week of ${wy}-${wm}-${wd}` };
		}
	}
	// Force exhaustive return.
	return { value: `${y}-${m}-${d}`, label: iso };
}

function numberBucket(n: number, step: number): { value: string; label: string } {
	const s = step > 0 ? step : 1;
	const lo = Math.floor(n / s) * s;
	const hi = lo + s;
	return {
		value: `[${lo},${hi})`,
		label: `${lo}–${hi}`
	};
}

function firstLetterBucket(s: string): { value: string; label: string } | null {
	const trimmed = s.trim();
	if (!trimmed) return null;
	const ch = trimmed[0].toUpperCase();
	return { value: /^[A-Z]$/.test(ch) ? ch : '#', label: /^[A-Z]$/.test(ch) ? ch : '#' };
}

function asStringList(value: unknown): string[] {
	if (Array.isArray(value)) return value.filter((v): v is string => typeof v === 'string');
	return [];
}

function wikilinkTarget(s: string): string | null {
	const m = /^\[\[(.+)\]\]$/.exec(s.trim());
	return m ? m[1] : null;
}

/**
 * Build the buckets for a `groupBy` property over `items`. The
 * returned list keeps the schema's option order for select-typed
 * properties; uncategorized items land in a final implicit bucket
 * when present.
 */
export function groupItems(
	items: EntityListItem[],
	details: Record<string, { entity: Record<string, unknown> } | null>,
	spec: GroupSpec
): Bucket[] {
	const { property } = spec;
	const buckets =
		property.type === 'single-select' || property.type === 'multi-select'
			? fromSelect(property as SingleSelectProperty | MultiSelectProperty)
			: new Map<string, Bucket>();
	const uncategorized: EntityListItem[] = [];

	for (const item of items) {
		const v = readField(details, item, property.key);

		if (property.type === 'single-select') {
			if (typeof v === 'string' && buckets.has(v)) {
				buckets.get(v)!.items.push(item);
			} else {
				uncategorized.push(item);
			}
		} else if (property.type === 'multi-select') {
			const xs = asStringList(v);
			let any = false;
			for (const x of xs) {
				if (buckets.has(x)) {
					buckets.get(x)!.items.push(item);
					any = true;
				}
			}
			if (!any) uncategorized.push(item);
		} else if (property.type === 'date') {
			if (typeof v !== 'string') {
				uncategorized.push(item);
				continue;
			}
			const b = dateBucket(v, spec.dateGranularity ?? 'month');
			if (!b) uncategorized.push(item);
			else ensure(buckets, b.value, b.label, 'gray').items.push(item);
		} else if (property.type === 'number') {
			if (typeof v !== 'number') {
				uncategorized.push(item);
				continue;
			}
			const b = numberBucket(v, spec.numberStep ?? 1);
			ensure(buckets, b.value, b.label, 'gray').items.push(item);
		} else if (property.type === 'relation') {
			// Single wikilink string OR list of them — extract target
			// id, bucket by it. Bucket label defaults to the id;
			// GroupedList may swap in a resolved entity name.
			const links = typeof v === 'string' ? [v] : asStringList(v);
			let any = false;
			for (const link of links) {
				const target = wikilinkTarget(link);
				if (!target) continue;
				ensure(buckets, target, target, 'gray').items.push(item);
				any = true;
			}
			if (!any) uncategorized.push(item);
		} else {
			// text / url / fallback — first-letter buckets
			const s = typeof v === 'string' ? v : '';
			const b = firstLetterBucket(s);
			if (!b) uncategorized.push(item);
			else ensure(buckets, b.value, b.label, 'gray').items.push(item);
		}
	}

	const out: Bucket[] = [];
	// For select-typed properties, preserve schema option order even
	// for empty buckets — useful on Board where the user expects the
	// columns to appear consistently.
	if (property.type === 'single-select' || property.type === 'multi-select') {
		const opts = (property as SingleSelectProperty | MultiSelectProperty).options;
		for (const o of opts) {
			out.push(buckets.get(o.value)!);
		}
	} else {
		// For derived buckets, sort by the underlying value:
		//  - number buckets `[lo,hi)` → numeric sort by lo (lex-sort
		//    would put "[10,15)" before "[5,10)")
		//  - dates / letters lex-sort fine as strings.
		const isNumberBucket = property.type === 'number';
		const numericLo = (s: string): number => {
			const m = /^\[(-?\d+(?:\.\d+)?)/.exec(s);
			return m ? Number(m[1]) : Number.NaN;
		};
		const sorted = Array.from(buckets.values()).sort((a, b) => {
			if (isNumberBucket) {
				return numericLo(a.value) - numericLo(b.value);
			}
			return a.value < b.value ? -1 : a.value > b.value ? 1 : 0;
		});
		out.push(...sorted);
	}
	if (uncategorized.length > 0) {
		out.push({ ...UNCATEGORIZED, items: uncategorized });
	}
	return out;
}

/** Properties the user can group by — every supported property type.
 * `relation` is included; bucket value is the wikilink target id and
 * GroupedList resolves it to the entity name for the bucket label. */
export function groupableFrom(properties: Property[]): Property[] {
	return properties.filter((p) =>
		['single-select', 'multi-select', 'date', 'number', 'text', 'url', 'relation'].includes(p.type)
	);
}
