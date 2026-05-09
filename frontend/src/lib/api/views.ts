/**
 * Saved-views API client.
 */

import { apiFetch } from './client';
import type { EntityType } from '../types';
import type { TypeViews, View, ViewKind, ViewsFile } from '../types/view';

export interface CreateViewBody {
	name: string;
	kind?: ViewKind;
	filter?: View['filter'];
	sort_key?: string | null;
	sort_dir?: 'asc' | 'desc';
	group_by_key?: string | null;
	columns?: string[] | null;
	card_properties?: string[] | null;
}

export interface UpdateViewBody {
	name: string;
	kind: ViewKind;
	filter: View['filter'];
	sort_key?: string | null;
	sort_dir?: 'asc' | 'desc';
	group_by_key?: string | null;
	columns?: string[] | null;
	card_properties?: string[] | null;
}

export function getAllViews(): Promise<ViewsFile> {
	return apiFetch('/api/views');
}

export function getViewsForType(type: EntityType): Promise<TypeViews> {
	return apiFetch(`/api/views/types/${type}`);
}

export function createView(
	type: EntityType,
	body: CreateViewBody
): Promise<{ view: View; type_views: TypeViews }> {
	return apiFetch(`/api/views/types/${type}`, {
		method: 'POST',
		body: JSON.stringify(body)
	});
}

export function updateView(
	type: EntityType,
	viewId: string,
	body: UpdateViewBody
): Promise<{ view: View; type_views: TypeViews }> {
	return apiFetch(`/api/views/types/${type}/${encodeURIComponent(viewId)}`, {
		method: 'PUT',
		body: JSON.stringify(body)
	});
}

export function deleteView(type: EntityType, viewId: string): Promise<TypeViews> {
	return apiFetch(`/api/views/types/${type}/${encodeURIComponent(viewId)}`, {
		method: 'DELETE'
	});
}

export function reorderViews(type: EntityType, ordered_ids: string[]): Promise<TypeViews> {
	return apiFetch(`/api/views/types/${type}/reorder`, {
		method: 'POST',
		body: JSON.stringify({ ordered_ids })
	});
}
