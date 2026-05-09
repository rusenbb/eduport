/**
 * Reactive cache of the user-managed property schema.
 *
 * The schema rarely changes, so we load it once on app boot and refetch
 * after any mutation. Components subscribe to read; mutators round-trip
 * through the API and update the store on success.
 */

import { writable } from 'svelte/store';
import {
	addProperty as apiAdd,
	applyTierTemplate as apiTierTemplate,
	deleteProperty as apiDelete,
	getSchema,
	getTypeSchema,
	patchProperty as apiPatch,
	reorderProperties as apiReorder,
	type PropertyPatch
} from '../api/schema';
import type { EntityType } from '../types';
import type { EntityTypeSchema, FullSchema, Property } from '../types/schema';

interface SchemaState {
	schema: FullSchema | null;
	loading: boolean;
	error: string | null;
}

const { subscribe, update, set } = writable<SchemaState>({
	schema: null,
	loading: false,
	error: null
});

let inflightLoad: Promise<FullSchema> | null = null;

async function load(): Promise<FullSchema> {
	if (inflightLoad) return inflightLoad;
	update((s) => ({ ...s, loading: true, error: null }));
	inflightLoad = (async () => {
		try {
			const schema = await getSchema();
			set({ schema, loading: false, error: null });
			return schema;
		} catch (e) {
			const message = e instanceof Error ? e.message : String(e);
			set({ schema: null, loading: false, error: message });
			throw e;
		} finally {
			inflightLoad = null;
		}
	})();
	return inflightLoad;
}

function mergeTypeSchema(typeSchema: EntityTypeSchema): void {
	update((s) => {
		if (!s.schema) return s;
		const next: FullSchema = {
			...s.schema,
			types: { ...s.schema.types, [typeSchema.entity_type]: typeSchema }
		};
		return { ...s, schema: next };
	});
}

export const schemaStore = {
	subscribe,
	load,
	async refresh(type?: EntityType): Promise<void> {
		if (type) {
			mergeTypeSchema(await getTypeSchema(type));
		} else {
			await load();
		}
	},
	async addProperty(type: EntityType, prop: Property): Promise<void> {
		mergeTypeSchema(await apiAdd(type, prop));
	},
	async patchProperty(type: EntityType, key: string, patch: PropertyPatch): Promise<void> {
		mergeTypeSchema(await apiPatch(type, key, patch));
	},
	async deleteProperty(type: EntityType, key: string): Promise<void> {
		mergeTypeSchema(await apiDelete(type, key));
	},
	async reorderProperties(type: EntityType, orderedKeys: string[]): Promise<void> {
		mergeTypeSchema(await apiReorder(type, orderedKeys));
	},
	async applyTierTemplate(types: EntityType[]): Promise<{ added: EntityType[]; existed: EntityType[] }> {
		const result = await apiTierTemplate(types);
		set({ schema: result.schema, loading: false, error: null });
		const added: EntityType[] = [];
		const existed: EntityType[] = [];
		for (const [t, info] of Object.entries(result.results)) {
			(info.status === 'added' ? added : existed).push(t as EntityType);
		}
		return { added, existed };
	}
};

/** Read-only convenience: synchronously snapshot the cached schema (or null). */
export function getCachedSchema(): FullSchema | null {
	let snap: FullSchema | null = null;
	const unsub = subscribe((s) => {
		snap = s.schema;
	});
	unsub();
	return snap;
}

export function propertiesFor(schema: FullSchema | null, type: EntityType): Property[] {
	return schema?.types[type]?.properties ?? [];
}
