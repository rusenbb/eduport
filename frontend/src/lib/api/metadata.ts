import { coreInvoke } from './client';
import type { EntityType } from '../types';

export function getCounts(): Promise<Partial<Record<EntityType, number>>> {
	return coreInvoke('core_get_counts');
}

export function getTags(): Promise<{ tag: string; count: number }[]> {
	return coreInvoke('core_get_tags');
}
