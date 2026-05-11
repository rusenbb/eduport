/**
 * Aggregator that returns every entity in the vault across all types,
 * with a short TTL cache so the slash menu / wikilink autocomplete
 * doesn't issue 8 backend calls per keystroke.
 */
import { listEntities } from '$lib/api/entities';
import { ENTITY_TYPES, type EntityListItem, type EntityType } from '$lib/types';

let cache: { at: number; items: { item: EntityListItem; type: EntityType }[] } | null = null;
const TTL_MS = 5_000;

export async function allEntities(): Promise<{ item: EntityListItem; type: EntityType }[]> {
	const now = Date.now();
	if (cache && now - cache.at < TTL_MS) return cache.items;
	const lists = await Promise.all(
		ENTITY_TYPES.map(async (type) => {
			try {
				const items = await listEntities(type);
				return items.map((item) => ({ item, type }));
			} catch {
				return [];
			}
		})
	);
	const items = lists.flat();
	cache = { at: now, items };
	return items;
}

export function invalidateEntityCache(): void {
	cache = null;
}
