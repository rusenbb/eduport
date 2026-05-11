/**
 * Notion-style compound filter (Phase B).
 *
 * Mirrors the Rust `FilterTree` / `FilterNode` / `FilterCondition`
 * types in `crates/eduport-core/src/view/filter_tree.rs`. The wire
 * format is exactly what the Rust side accepts via `serde_json` —
 * snake_case operator names, lowercase combinator, untagged value.
 *
 * The tree is *additive* alongside the legacy flat ViewFilter — a
 * View can carry both (merged with AND in the query adapter), so
 * existing chip filters keep working while the new compound filter
 * builds on top.
 */

export type Combinator = 'and' | 'or';

export type FilterOperator =
	| 'equals'
	| 'not_equals'
	| 'is_empty'
	| 'is_not_empty'
	// text
	| 'contains'
	| 'not_contains'
	| 'starts_with'
	| 'ends_with'
	// number / date
	| 'gt'
	| 'gte'
	| 'lt'
	| 'lte'
	// multi-select / list
	| 'contains_any'
	| 'does_not_contain';

/** Untagged value: string | number | boolean | string[] — matches the
 * Rust `FilterValue` serde-untagged enum. */
export type FilterValue = string | number | boolean | string[];

export interface FilterCondition {
	kind: 'cond';
	property_key: string;
	operator: FilterOperator;
	value?: FilterValue;
}

export interface FilterGroup {
	kind: 'group';
	op: Combinator;
	children: FilterNode[];
}

export type FilterNode = FilterGroup | FilterCondition;

export interface FilterTree {
	root?: FilterNode | null;
}

// ── Operator catalogue per property type ────────────────────────────

import type { PropertyType } from './schema';

export interface OperatorMeta {
	op: FilterOperator;
	label: string;
	/** Whether the operator takes a value (false → unary like is_empty). */
	hasValue: boolean;
}

const COMMON_NULLABLE: OperatorMeta[] = [
	{ op: 'is_empty', label: 'is empty', hasValue: false },
	{ op: 'is_not_empty', label: 'is not empty', hasValue: false }
];

export const OPERATORS_BY_TYPE: Record<PropertyType, OperatorMeta[]> = {
	text: [
		{ op: 'equals', label: 'equals', hasValue: true },
		{ op: 'not_equals', label: 'does not equal', hasValue: true },
		{ op: 'contains', label: 'contains', hasValue: true },
		{ op: 'not_contains', label: 'does not contain', hasValue: true },
		{ op: 'starts_with', label: 'starts with', hasValue: true },
		{ op: 'ends_with', label: 'ends with', hasValue: true },
		...COMMON_NULLABLE
	],
	number: [
		{ op: 'equals', label: '=', hasValue: true },
		{ op: 'not_equals', label: '≠', hasValue: true },
		{ op: 'gt', label: '>', hasValue: true },
		{ op: 'gte', label: '≥', hasValue: true },
		{ op: 'lt', label: '<', hasValue: true },
		{ op: 'lte', label: '≤', hasValue: true },
		...COMMON_NULLABLE
	],
	date: [
		{ op: 'equals', label: 'is', hasValue: true },
		{ op: 'gt', label: 'after', hasValue: true },
		{ op: 'gte', label: 'on or after', hasValue: true },
		{ op: 'lt', label: 'before', hasValue: true },
		{ op: 'lte', label: 'on or before', hasValue: true },
		...COMMON_NULLABLE
	],
	checkbox: [
		{ op: 'equals', label: 'is', hasValue: true }
	],
	'single-select': [
		{ op: 'equals', label: 'is', hasValue: true },
		{ op: 'not_equals', label: 'is not', hasValue: true },
		...COMMON_NULLABLE
	],
	'multi-select': [
		{ op: 'contains_any', label: 'contains any of', hasValue: true },
		{ op: 'does_not_contain', label: 'does not contain', hasValue: true },
		...COMMON_NULLABLE
	],
	url: [
		{ op: 'equals', label: 'equals', hasValue: true },
		{ op: 'contains', label: 'contains', hasValue: true },
		...COMMON_NULLABLE
	],
	relation: [
		{ op: 'equals', label: 'is', hasValue: true },
		...COMMON_NULLABLE
	]
};

export function operatorsFor(type: PropertyType): OperatorMeta[] {
	return OPERATORS_BY_TYPE[type] ?? [];
}

export function defaultOperatorFor(type: PropertyType): FilterOperator {
	return operatorsFor(type)[0]?.op ?? 'equals';
}

/** Empty single-group tree: `(AND with no conditions)`. Translator
 * yields no expr for this (degrades to no-op). */
export function emptyTree(): FilterTree {
	return { root: { kind: 'group', op: 'and', children: [] } };
}

/** True if the tree has any conditions at all. */
export function treeHasConditions(tree: FilterTree | null | undefined): boolean {
	if (!tree?.root) return false;
	const visit = (n: FilterNode): boolean =>
		n.kind === 'cond' ? true : n.children.some(visit);
	return visit(tree.root);
}
