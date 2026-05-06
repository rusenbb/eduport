import { apiFetch } from './client';
import type { TrashItem } from '../types';

export function listTrash(): Promise<TrashItem[]> {
	return apiFetch('/trash');
}

export function restoreTrashItem(name: string): Promise<{ path: string; file_id: string }> {
	return apiFetch('/trash/restore', {
		method: 'POST',
		body: JSON.stringify({ name })
	});
}

export function deleteTrashItem(name: string): Promise<void> {
	return apiFetch(`/trash/${encodeURIComponent(name)}`, { method: 'DELETE' });
}

export function emptyTrash(): Promise<void> {
	return apiFetch('/trash', { method: 'DELETE' });
}
