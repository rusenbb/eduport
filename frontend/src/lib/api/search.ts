import { apiFetch } from './client';
import type { SearchHit } from '../types';

export function search(q: string, limit = 50): Promise<SearchHit[]> {
	if (!q.trim()) return Promise.resolve([]);
	return apiFetch(`/search?q=${encodeURIComponent(q)}&limit=${limit}`);
}
