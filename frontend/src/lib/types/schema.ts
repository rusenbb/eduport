/**
 * Schema and custom-property types — mirror the sidecar's models/schema.py
 * discriminated union. The on-the-wire shape is exactly what the sidecar
 * sends (JSON-serialized Pydantic).
 */

import type { EntityType } from '../types';

export type PropertyType =
	| 'text'
	| 'number'
	| 'date'
	| 'checkbox'
	| 'single-select'
	| 'multi-select'
	| 'url'
	| 'relation';

export type OptionColor =
	| 'gray'
	| 'red'
	| 'orange'
	| 'yellow'
	| 'green'
	| 'teal'
	| 'blue'
	| 'purple'
	| 'pink';

export interface SelectOption {
	value: string;
	label: string;
	color: OptionColor;
}

interface PropertyBase {
	key: string;
	name: string;
	description?: string | null;
	required?: boolean;
	/** True for system-seeded properties (country, role, status, …)
	 * that ship with eduport. Built-ins cannot be deleted and their
	 * `type` is immutable, but their option lists, name, and
	 * description can be edited like any user-defined property. */
	is_builtin?: boolean;
}

export interface TextProperty extends PropertyBase {
	type: 'text';
	default?: string | null;
}

export interface NumberProperty extends PropertyBase {
	type: 'number';
	unit?: string | null;
	default?: number | null;
}

export interface DateProperty extends PropertyBase {
	type: 'date';
	default?: string | null;
}

export interface CheckboxProperty extends PropertyBase {
	type: 'checkbox';
	default?: boolean | null;
}

export interface SingleSelectProperty extends PropertyBase {
	type: 'single-select';
	options: SelectOption[];
	default?: string | null;
}

export interface MultiSelectProperty extends PropertyBase {
	type: 'multi-select';
	options: SelectOption[];
	default?: string[] | null;
}

export interface UrlProperty extends PropertyBase {
	type: 'url';
	default?: string | null;
}

export interface RelationProperty extends PropertyBase {
	type: 'relation';
	target_types?: EntityType[] | null;
	default?: string | null; // wikilink string [[target-id]]
}

export type Property =
	| TextProperty
	| NumberProperty
	| DateProperty
	| CheckboxProperty
	| SingleSelectProperty
	| MultiSelectProperty
	| UrlProperty
	| RelationProperty;

// `entity_type` is not in the Rust serialization (the map key in
// `FullSchema.types` carries it); `builtin_keys` was a legacy field
// from the Python sidecar that the Rust side stopped emitting.
// Keeping `builtin_keys` here as optional so any consumer that still
// reads it gets `undefined` instead of a hard error.
export interface EntityTypeSchema {
	builtin_keys?: string[];
	properties: Property[];
}

export interface FullSchema {
	version: number;
	types: Record<EntityType, EntityTypeSchema>;
}

/** Warning kinds emitted by the lenient custom-field validator on entity GET. */
export type WarningKind =
	| 'orphaned'
	| 'type_mismatch'
	| 'out_of_options'
	| 'broken_link'
	| 'wrong_target_type'
	| 'required_missing';

export interface ValueWarning {
	key: string;
	kind: WarningKind;
	message: string;
	value?: unknown;
}

/** Per-property aggregation entry returned by GET /api/properties/counts/... */
export interface PropertyCount {
	type: PropertyType;
	value: string; // for checkbox: "true" / "false"
	count: number;
}

/** Filter shape used by the list view; serialized to query string by api/properties.ts. */
export interface PropertyFilters {
	text: Record<string, string>;
	num: Record<string, [number | null, number | null]>;
	date: Record<string, [string | null, string | null]>;
	sort?: string;
	sortDir?: 'asc' | 'desc';
}

export const DEFAULT_PROPERTY_FILTERS: PropertyFilters = {
	text: {},
	num: {},
	date: {}
};

export const COLOR_PALETTE: OptionColor[] = [
	'gray',
	'red',
	'orange',
	'yellow',
	'green',
	'teal',
	'blue',
	'purple',
	'pink'
];

/** Tailwind-friendly classes for option chips. */
export const COLOR_CLASSES: Record<OptionColor, { bg: string; text: string; border: string }> = {
	gray: { bg: 'bg-gray-500/15', text: 'text-gray-200', border: 'border-gray-400/40' },
	red: { bg: 'bg-red-500/15', text: 'text-red-200', border: 'border-red-400/40' },
	orange: { bg: 'bg-orange-500/15', text: 'text-orange-200', border: 'border-orange-400/40' },
	yellow: { bg: 'bg-yellow-500/15', text: 'text-yellow-100', border: 'border-yellow-400/40' },
	green: { bg: 'bg-green-500/15', text: 'text-green-200', border: 'border-green-400/40' },
	teal: { bg: 'bg-teal-500/15', text: 'text-teal-200', border: 'border-teal-400/40' },
	blue: { bg: 'bg-blue-500/15', text: 'text-blue-200', border: 'border-blue-400/40' },
	purple: { bg: 'bg-purple-500/15', text: 'text-purple-200', border: 'border-purple-400/40' },
	pink: { bg: 'bg-pink-500/15', text: 'text-pink-200', border: 'border-pink-400/40' }
};
