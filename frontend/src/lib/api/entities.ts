import { coreInvoke } from './client';
import type { EntityDetail, EntityListItem, EntityType } from '../types';

export function listEntities(type: EntityType, tags?: string[]): Promise<EntityListItem[]> {
	return coreInvoke('core_entity_list', {
		entityType: type,
		tags: tags && tags.length > 0 ? tags : null
	});
}

export function getEntity(type: EntityType, fileId: string): Promise<EntityDetail> {
	return coreInvoke('core_entity_get', { entityType: type, fileId });
}

export function resolveEntity(target: string): Promise<{ file_id: string; type: EntityType; name: string }> {
	return coreInvoke('core_entity_resolve', { target });
}

export function createEntity(
	type: EntityType,
	frontmatter: Record<string, unknown>,
	body = ''
): Promise<{ file_id: string }> {
	return coreInvoke('core_entity_create', { entityType: type, frontmatter, body });
}

export function updateEntity(
	type: EntityType,
	fileId: string,
	frontmatter: Record<string, unknown>,
	body = ''
): Promise<{ file_id: string }> {
	return coreInvoke('core_entity_update', { entityType: type, fileId, frontmatter, body });
}

export function deleteEntity(type: EntityType, fileId: string): Promise<void> {
	return coreInvoke('core_entity_delete', { entityType: type, fileId });
}
