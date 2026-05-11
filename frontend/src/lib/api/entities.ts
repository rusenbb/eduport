import { commands, type JsonValue } from '../bindings';
import type { EntityDetail, EntityListItem, EntityType } from '../types';
import { unwrap } from './client';

export function listEntities(type: EntityType, tags?: string[]): Promise<EntityListItem[]> {
	return unwrap(commands.coreEntityList(type, tags && tags.length > 0 ? tags : null)) as Promise<
		EntityListItem[]
	>;
}

export function getEntity(type: EntityType, fileId: string): Promise<EntityDetail> {
	return unwrap(commands.coreEntityGet(type, fileId)) as Promise<EntityDetail>;
}

export function resolveEntity(
	target: string
): Promise<{ file_id: string; type: EntityType; name: string }> {
	return unwrap(commands.coreEntityResolve(target)) as Promise<{
		file_id: string;
		type: EntityType;
		name: string;
	}>;
}

export function createEntity(
	type: EntityType,
	frontmatter: Record<string, unknown>,
	body = ''
): Promise<{ file_id: string }> {
	return unwrap(commands.coreEntityCreate(type, frontmatter as unknown as JsonValue, body));
}

export function updateEntity(
	type: EntityType,
	fileId: string,
	frontmatter: Record<string, unknown>,
	body = ''
): Promise<{ file_id: string }> {
	return unwrap(
		commands.coreEntityUpdate(type, fileId, frontmatter as unknown as JsonValue, body)
	);
}

export function deleteEntity(type: EntityType, fileId: string): Promise<void> {
	return unwrap(commands.coreEntityDelete(type, fileId)).then(() => undefined);
}

/**
 * List entities whose `parent` frontmatter field equals `parentFileId`.
 * Cross-type — sub-pages aren't constrained to the parent's type.
 */
export function entityChildren(parentFileId: string): Promise<EntityListItem[]> {
	return unwrap(commands.coreEntityChildren(parentFileId)) as Promise<EntityListItem[]>;
}
