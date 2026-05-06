import { apiFetch } from './client';
import type { EntityType } from '../types';

export function getCounts(): Promise<Partial<Record<EntityType, number>>> {
	return apiFetch('/counts');
}

export function getTags(): Promise<{ tag: string; count: number }[]> {
	return apiFetch('/tags');
}
