import { commands } from '../bindings';
import type { TrashItem } from '../types';
import { unwrap } from './client';

export function listTrash(): Promise<TrashItem[]> {
	return unwrap(commands.coreTrashList()) as Promise<TrashItem[]>;
}

export function restoreTrashItem(name: string): Promise<{ path: string; file_id: string }> {
	return unwrap(commands.coreTrashRestore({ name }));
}

export function deleteTrashItem(name: string): Promise<void> {
	return unwrap(commands.coreTrashDelete(name)).then(() => undefined);
}

export function emptyTrash(): Promise<void> {
	return unwrap(commands.coreTrashEmpty()).then(() => undefined);
}
