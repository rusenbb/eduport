import { coreInvoke } from './client';
import type { SearchHit } from '../types';

export function search(q: string, limit = 50, tags: string[] = []): Promise<SearchHit[]> {
	if (!q.trim()) return Promise.resolve([]);
	return coreInvoke('core_search', { q, limit, tags });
}
