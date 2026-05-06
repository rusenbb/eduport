import { apiFetch } from './client';
import type { EntityDetail, EntityListItem, EntityType } from '../types';

export function listEntities(type: EntityType, tags?: string[]): Promise<EntityListItem[]> {
	const qs = tags && tags.length > 0 ? '?' + tags.map((t) => `tag=${encodeURIComponent(t)}`).join('&') : '';
	return apiFetch(`/entities/${type}${qs}`);
}

export function getEntity(type: EntityType, fileId: string): Promise<EntityDetail> {
	return apiFetch(`/entities/${type}/${encodeURIComponent(fileId)}`);
}

export function resolveEntity(target: string): Promise<{ file_id: string; type: EntityType; name: string }> {
	return apiFetch(`/entities/resolve/${encodeURIComponent(target)}`);
}

export function createEntity(
	type: EntityType,
	frontmatter: Record<string, unknown>,
	body = ''
): Promise<{ file_id: string }> {
	return apiFetch(`/entities/${type}`, {
		method: 'POST',
		body: JSON.stringify({ frontmatter, body })
	});
}

export function updateEntity(
	type: EntityType,
	fileId: string,
	frontmatter: Record<string, unknown>,
	body = ''
): Promise<{ file_id: string }> {
	return apiFetch(`/entities/${type}/${encodeURIComponent(fileId)}`, {
		method: 'PATCH',
		body: JSON.stringify({ frontmatter, body })
	});
}

export function deleteEntity(type: EntityType, fileId: string): Promise<void> {
	return apiFetch(`/entities/${type}/${encodeURIComponent(fileId)}`, { method: 'DELETE' });
}
