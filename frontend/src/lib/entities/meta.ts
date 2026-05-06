import type { EntityDetail, EntityListItem, EntityType } from '$lib/types';

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

export function readField(entity: Record<string, unknown>, key: string): string {
	const value = entity[key];
	if (value === null || value === undefined) return '';
	if (Array.isArray(value)) return value.join(', ');
	return String(value);
}

export function summarizeDetail(detail: EntityDetail): string {
	const e = detail.entity;
	switch (detail.type) {
		case 'program':
			return [readField(e, 'level'), readField(e, 'deadline'), targetOf(e.university) ?? '']
				.filter(Boolean)
				.join(' · ');
		case 'application':
			return [readField(e, 'status'), readField(e, 'internal_deadline'), targetOf(e.program) ?? '']
				.filter(Boolean)
				.join(' · ');
		case 'person':
			return [readField(e, 'role'), readField(e, 'email'), targetOf(e.university) ?? '']
				.filter(Boolean)
				.join(' · ');
		case 'document':
			return [readField(e, 'status'), readField(e, 'date'), readField(e, 'file')]
				.filter(Boolean)
				.join(' · ');
		case 'email':
			return [readField(e, 'direction'), readField(e, 'date'), readField(e, 'subject')]
				.filter(Boolean)
				.join(' · ');
		default:
			return userTags(e, detail.type).join(' · ');
	}
}

export function summarizeItem(item: EntityListItem, detail?: EntityDetail | null): string {
	if (detail) return summarizeDetail(detail);
	return item.file_id;
}
