/**
 * Saved-views API client. Phase-10 cutover: Tauri command channel.
 */

import { coreInvoke } from './client';
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
	return coreInvoke('core_view_get_all');
}

export function getViewsForType(type: EntityType): Promise<TypeViews> {
	return coreInvoke('core_view_get_for_type', { entityType: type });
}

export function createView(
	type: EntityType,
	body: CreateViewBody
): Promise<{ view: View; type_views: TypeViews }> {
	return coreInvoke('core_view_create', { entityType: type, body });
}

export function updateView(
	type: EntityType,
	viewId: string,
	body: UpdateViewBody
): Promise<{ view: View; type_views: TypeViews }> {
	return coreInvoke('core_view_update', { entityType: type, viewId, body });
}

export function deleteView(type: EntityType, viewId: string): Promise<TypeViews> {
	return coreInvoke('core_view_delete', { entityType: type, viewId });
}

export function reorderViews(type: EntityType, ordered_ids: string[]): Promise<TypeViews> {
	return coreInvoke('core_view_reorder', { entityType: type, orderedIds: ordered_ids });
}
