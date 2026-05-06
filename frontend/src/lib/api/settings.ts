import { apiFetch } from './client';
import type { Settings } from '../types';

export function getSettings(): Promise<Settings> {
	return apiFetch('/settings');
}

export function putSettings(settings: Settings): Promise<Settings> {
	return apiFetch('/settings', { method: 'PUT', body: JSON.stringify(settings) });
}
