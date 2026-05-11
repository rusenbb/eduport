import { commands } from '../bindings';
import type { SearchHit } from '../types';
import { unwrap } from './client';

export function search(q: string, limit = 50, tags: string[] = []): Promise<SearchHit[]> {
	if (!q.trim()) return Promise.resolve([]);
	return unwrap(commands.coreSearch(q, limit, tags)) as Promise<SearchHit[]>;
}
