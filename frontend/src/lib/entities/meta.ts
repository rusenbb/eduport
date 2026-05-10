import type { EntityDetail, EntityListItem, EntityType } from '$lib/types';
import type { Property } from '$lib/types/schema';

export interface FieldDef {
	key: string;
	label: string;
	kind: 'text' | 'email' | 'url' | 'date' | 'select' | 'wikilink' | 'wikilinks' | 'resources';
	options?: string[];
	linkType?: EntityType;
	placeholder?: string;
}

export const TYPE_LABELS: Record<EntityType, string> = {
	university: 'University',
	lab: 'Lab',
	person: 'Person',
	program: 'Program',
	application: 'Application',
	document: 'Document',
	email: 'Email',
	note: 'Note'
};

export const TYPE_TAGS: Record<EntityType, string> = {
	university: 'eduport-type/university',
	lab: 'eduport-type/lab',
	person: 'eduport-type/person',
	program: 'eduport-type/program',
	application: 'eduport-type/application',
	document: 'eduport-type/document',
	email: 'eduport-type/email',
	note: 'eduport-type/note'
};

export const FIELD_DEFS: Record<EntityType, FieldDef[]> = {
	university: [
		{ key: 'country', label: 'Country', kind: 'text' },
		{ key: 'city', label: 'City', kind: 'text' },
		{ key: 'website', label: 'Website', kind: 'url' },
		{ key: 'links', label: 'Links', kind: 'resources' },
		{ key: 'emails', label: 'Emails', kind: 'resources' }
	],
	lab: [
		{ key: 'focus', label: 'Focus', kind: 'text' },
		{ key: 'website', label: 'Website', kind: 'url' },
		{ key: 'university', label: 'University', kind: 'wikilink', linkType: 'university' },
		{ key: 'links', label: 'Links', kind: 'resources' },
		{ key: 'emails', label: 'Emails', kind: 'resources' }
	],
	person: [
		{ key: 'role', label: 'Role', kind: 'text' },
		{ key: 'email', label: 'Email', kind: 'email' },
		{ key: 'website', label: 'Website', kind: 'url' },
		{ key: 'university', label: 'University', kind: 'wikilink', linkType: 'university' },
		{ key: 'labs', label: 'Labs', kind: 'wikilinks', linkType: 'lab' },
		{ key: 'links', label: 'Links', kind: 'resources' }
	],
	program: [
		{ key: 'level', label: 'Level', kind: 'select', options: ['undergrad', 'masters', 'phd'] },
		{ key: 'department', label: 'Department', kind: 'text' },
		{ key: 'language', label: 'Language', kind: 'text' },
		{ key: 'duration', label: 'Duration', kind: 'text' },
		{ key: 'deadline', label: 'Deadline', kind: 'date' },
		{ key: 'tuition', label: 'Tuition', kind: 'text' },
		{ key: 'website', label: 'Website', kind: 'url' },
		{ key: 'university', label: 'University', kind: 'wikilink', linkType: 'university' },
		{ key: 'people', label: 'People', kind: 'wikilinks', linkType: 'person' },
		{ key: 'links', label: 'Links', kind: 'resources' },
		{ key: 'emails', label: 'Emails', kind: 'resources' }
	],
	application: [
		{ key: 'program', label: 'Program', kind: 'wikilink', linkType: 'program' },
		{
			key: 'status',
			label: 'Status',
			kind: 'select',
			options: ['planning', 'drafting', 'submitted', 'decision-pending', 'accepted', 'rejected', 'withdrawn']
		},
		{ key: 'internal_deadline', label: 'Internal deadline', kind: 'date' },
		{ key: 'submitted_at', label: 'Submitted at', kind: 'date' },
		{ key: 'decision_at', label: 'Decision at', kind: 'date' },
		{ key: 'documents', label: 'Documents', kind: 'wikilinks', linkType: 'document' }
	],
	document: [
		{ key: 'title', label: 'Title', kind: 'text' },
		{ key: 'date', label: 'Date', kind: 'date' },
		{ key: 'file', label: 'File', kind: 'text', placeholder: 'attachments/example.pdf' },
		{ key: 'status', label: 'Status', kind: 'select', options: ['requested', 'drafting', 'received'] },
		{ key: 'requested_at', label: 'Requested at', kind: 'date' },
		{ key: 'recommender', label: 'Recommender', kind: 'wikilink', linkType: 'person' }
	],
	email: [
		{ key: 'direction', label: 'Direction', kind: 'select', options: ['inbound', 'outbound'] },
		{ key: 'date', label: 'Date', kind: 'date' },
		{ key: 'subject', label: 'Subject', kind: 'text' },
		{ key: 'from', label: 'From', kind: 'email' },
		{ key: 'to', label: 'To', kind: 'text', placeholder: 'one@example.com, two@example.com' },
		{ key: 'cc', label: 'Cc', kind: 'text', placeholder: 'one@example.com, two@example.com' },
		{ key: 'bcc', label: 'Bcc', kind: 'text', placeholder: 'one@example.com, two@example.com' },
		{ key: 'related_program', label: 'Related program', kind: 'wikilink', linkType: 'program' },
		{ key: 'related_application', label: 'Related application', kind: 'wikilink', linkType: 'application' },
		{ key: 'related_people', label: 'Related people', kind: 'wikilinks', linkType: 'person' },
		{ key: 'in_reply_to', label: 'In reply to', kind: 'wikilink', linkType: 'email' },
		{ key: 'attachments', label: 'Attachments', kind: 'wikilinks', linkType: 'document' }
	],
	note: []
};

/**
 * Built-in fields that make sense to filter on, surfaced as synthetic
 * `Property` records so they slot into the existing PropertyFilterBar
 * UI alongside user-declared custom properties.
 *
 * The Rust index only ever knows about custom properties — built-in
 * fields live in the entity's frontmatter and aren't in the
 * `properties` SQLite table. We split filters on the call site
 * (see `EntityWorkspace.loadList`): backend gets the custom keys,
 * built-in keys are post-filtered in memory against the already-
 * fetched detail records.
 *
 * Every type also gets a synthetic "Name contains" filter, since the
 * `name` field is universal and easier to remember than building a
 * search query.
 */
export function builtinFilterableProperties(type: EntityType): Property[] {
	const fields = FIELD_DEFS[type] ?? [];
	const filterable: Property[] = [
		{
			key: 'name',
			name: 'Name',
			description: 'Entity name (contains)',
			required: false,
			type: 'text'
		} as Property
	];
	for (const f of fields) {
		if (f.kind === 'text' || f.kind === 'email') {
			filterable.push({
				key: f.key,
				name: f.label,
				required: false,
				type: 'text'
			} as Property);
		} else if (f.kind === 'date') {
			filterable.push({
				key: f.key,
				name: f.label,
				required: false,
				type: 'date'
			} as Property);
		} else if (f.kind === 'select' && f.options) {
			filterable.push({
				key: f.key,
				name: f.label,
				required: false,
				type: 'single-select',
				options: f.options.map((o) => ({ value: o, label: o, color: 'gray' }))
			} as Property);
		}
	}
	return filterable;
}

/** Keys recognised as built-in (non-schema) filter fields, used by
 * loadList to split filter routing between backend (custom-property
 * indexed) and frontend (in-memory). */
export function builtinFilterKeys(type: EntityType): Set<string> {
	return new Set(builtinFilterableProperties(type).map((p) => p.key));
}

export function typeTag(type: EntityType): string {
	return TYPE_TAGS[type];
}

export function userTags(frontmatter: Record<string, unknown>, type: EntityType): string[] {
	const reserved = typeTag(type);
	return Array.isArray(frontmatter.tags)
		? (frontmatter.tags as string[]).filter((tag) => tag !== reserved)
		: [];
}

export function targetOf(link: unknown): string | null {
	return typeof link === 'string' && /^\[\[[^\]\[]+\]\]$/.test(link)
		? link.slice(2, -2).trim()
		: null;
}

export function asWikilink(fileId: string): string {
	return `[[${fileId}]]`;
}

export function inferTypeFromField(field: string, fallback: EntityType = 'note'): EntityType {
	const map: Record<string, EntityType> = {
		university: 'university',
		labs: 'lab',
		people: 'person',
		program: 'program',
		recommender: 'person',
		related_program: 'program',
		related_application: 'application',
		related_people: 'person',
		documents: 'document',
		attachments: 'document',
		in_reply_to: 'email'
	};
	return map[field] ?? fallback;
}

/**
 * Map a built-in field's `kind` (Pydantic-typed) onto the closest
 * `PropertyType` so the same `<PropertyTypeIcon>` covers both surfaces.
 *
 * The mapping is deliberately lossy: `email` → `text` (icon-wise it's just
 * a string), `wikilinks` and `resources` → list-style icons (`relation` and
 * `multi-select` respectively). The point isn't perfect fidelity, it's
 * visual consistency between custom + built-in rows.
 */
export function builtinKindToPropertyType(
	kind: FieldDef['kind']
): import('$lib/types/schema').PropertyType {
	switch (kind) {
		case 'text':
		case 'email':
			return 'text';
		case 'url':
			return 'url';
		case 'date':
			return 'date';
		case 'select':
			return 'single-select';
		case 'wikilink':
			return 'relation';
		case 'wikilinks':
			return 'relation';
		case 'resources':
			return 'multi-select';
	}
}

/** Display label for a built-in kind, mirroring the custom property types. */
export const BUILTIN_KIND_LABELS: Record<FieldDef['kind'], string> = {
	text: 'Text',
	email: 'Email',
	url: 'URL',
	date: 'Date',
	select: 'Single select',
	wikilink: 'Relation',
	wikilinks: 'Relations (list)',
	resources: 'Resources (list)'
};

export function readField(entity: Record<string, unknown>, key: string): string {
	const value = entity[key];
	if (value === null || value === undefined) return '';
	if (Array.isArray(value)) return value.join(', ');
	return String(value);
}

export function summarizeDetail(detail: EntityDetail): string {
	const e = detail.entity;
	const tags = userTags(e, detail.type);
	const tagSummary = tags.length > 0 ? '#' + tags.join(' #') : '';
	const join = (parts: (string | undefined | null)[]) =>
		parts.filter((p): p is string => !!p && p.length > 0).join(' · ');

	switch (detail.type) {
		case 'university':
			return join([readField(e, 'country'), readField(e, 'city'), tagSummary]);
		case 'lab':
			return join([readField(e, 'focus'), targetOf(e.university) ?? '', tagSummary]);
		case 'program':
			return join([readField(e, 'level'), readField(e, 'deadline'), targetOf(e.university) ?? '']);
		case 'application':
			return join([
				readField(e, 'status'),
				readField(e, 'internal_deadline'),
				targetOf(e.program) ?? ''
			]);
		case 'person':
			return join([readField(e, 'role'), readField(e, 'email'), targetOf(e.university) ?? '']);
		case 'document':
			return join([readField(e, 'status'), readField(e, 'date'), readField(e, 'file')]);
		case 'email':
			return join([readField(e, 'direction'), readField(e, 'date'), readField(e, 'subject')]);
		case 'note':
			return tagSummary;
		default:
			return tagSummary;
	}
}

export function summarizeItem(item: EntityListItem, detail?: EntityDetail | null): string {
	if (!detail) return ''; // file_id is the row's secondary line elsewhere; don't leak it as the "summary" placeholder
	const summary = summarizeDetail(detail);
	return summary;
}
