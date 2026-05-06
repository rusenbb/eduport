import { apiFetch } from './client';
import type { SearchHit } from '../types';

export function search(q: string, limit = 50, tags: string[] = []): Promise<SearchHit[]> {
	if (!q.trim()) return Promise.resolve([]);
	const qs = new URLSearchParams({ q, limit: String(limit) });
	for (const tag of tags) qs.append('tag', tag);
	return apiFetch(`/search?${qs.toString()}`);
}
