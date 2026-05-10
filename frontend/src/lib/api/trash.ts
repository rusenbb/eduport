import { coreInvoke } from './client';
import type { TrashItem } from '../types';

export function listTrash(): Promise<TrashItem[]> {
	return coreInvoke('core_trash_list');
}

export function restoreTrashItem(name: string): Promise<{ path: string; file_id: string }> {
	return coreInvoke('core_trash_restore', { body: { name } });
}

export function deleteTrashItem(name: string): Promise<void> {
	return coreInvoke('core_trash_delete', { name });
}

export function emptyTrash(): Promise<void> {
	return coreInvoke('core_trash_empty');
}
