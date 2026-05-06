// Mirrors a useful subset of the sidecar's Pydantic shapes.

export type EntityType =
	| 'university'
	| 'lab'
	| 'person'
	| 'program'
	| 'application'
	| 'document'
	| 'email'
	| 'note';

export const ENTITY_TYPES: EntityType[] = [
	'university',
	'lab',
	'person',
	'program',
	'application',
	'document',
	'email',
	'note'
];

export interface EntityListItem {
	file_id: string;
	type: EntityType;
	name: string;
	path: string;
	mtime_ns: number;
}

export interface Backlink {
	src_file_id: string;
	field: string;
}

export interface EntityDetail {
	file_id: string;
	type: EntityType;
	entity: Record<string, unknown>;
	body: string;
	backlinks: Backlink[];
}

export interface SearchHit {
	file_id: string;
	type: EntityType;
	name: string;
	snippet: string;
}

export type ApplicationStatus =
	| 'planning'
	| 'drafting'
	| 'submitted'
	| 'decision-pending'
	| 'accepted'
	| 'rejected'
	| 'withdrawn';

export interface Settings {
	data_folder: string;
	attachments_folder: string;
	notes_folder: string;
	theme: 'system' | 'light' | 'dark';
	user_email: string;
}

export interface ParsedEml {
	from: string;
	to: string[];
	cc: string[];
	bcc: string[];
	subject: string;
	date: string | null;
	body: string;
	direction: 'inbound' | 'outbound';
}
