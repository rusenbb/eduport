import { apiFetch } from './client';
import type { ParsedEml } from '../types';

export function parseEml(file: File | Blob, filename = 'message.eml'): Promise<ParsedEml> {
	const form = new FormData();
	form.append('file', file, filename);
	return apiFetch('/eml/parse', { method: 'POST', body: form });
}
