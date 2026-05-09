/**
 * Reactive cache of saved views, mirroring schemaStore's shape.
 */

import { writable } from 'svelte/store';
import {
	createView as apiCreate,
	deleteView as apiDelete,
	getAllViews,
	getViewsForType,
	reorderViews as apiReorder,
	updateView as apiUpdate,
	type CreateViewBody,
	type UpdateViewBody
} from '../api/views';
import type { EntityType } from '../types';
import type { TypeViews, View, ViewsFile } from '../types/view';

interface State {
	file: ViewsFile | null;
	loading: boolean;
	error: string | null;
}

const { subscribe, update, set } = writable<State>({
	file: null,
	loading: false,
	error: null
});

let inflight: Promise<ViewsFile> | null = null;

async function load(): Promise<ViewsFile> {
	if (inflight) return inflight;
	update((s) => ({ ...s, loading: true, error: null }));
	inflight = (async () => {
		try {
			const file = await getAllViews();
			set({ file, loading: false, error: null });
			return file;
		} catch (e) {
			set({ file: null, loading: false, error: e instanceof Error ? e.message : String(e) });
			throw e;
		} finally {
			inflight = null;
		}
	})();
	return inflight;
}

function mergeTypeViews(typeViews: TypeViews): void {
	update((s) => {
		if (!s.file) return s;
		const next: ViewsFile = {
			...s.file,
			types: { ...s.file.types, [typeViews.entity_type]: typeViews }
		};
		return { ...s, file: next };
	});
}

export const viewsStore = {
	subscribe,
	load,
	async refresh(type?: EntityType): Promise<void> {
		if (type) mergeTypeViews(await getViewsForType(type));
		else await load();
	},
	async create(type: EntityType, body: CreateViewBody): Promise<View> {
		const result = await apiCreate(type, body);
		mergeTypeViews(result.type_views);
		return result.view;
	},
	async update(type: EntityType, id: string, body: UpdateViewBody): Promise<View> {
		const result = await apiUpdate(type, id, body);
		mergeTypeViews(result.type_views);
		return result.view;
	},
	async delete(type: EntityType, id: string): Promise<void> {
		mergeTypeViews(await apiDelete(type, id));
	},
	async reorder(type: EntityType, orderedIds: string[]): Promise<void> {
		mergeTypeViews(await apiReorder(type, orderedIds));
	}
};
