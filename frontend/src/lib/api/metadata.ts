import { commands } from '../bindings';
import type { EntityType } from '../types';
import { unwrap } from './client';

export function getCounts(): Promise<Partial<Record<EntityType, number>>> {
	return unwrap(commands.coreGetCounts()) as Promise<Partial<Record<EntityType, number>>>;
}

export function getTags(): Promise<{ tag: string; count: number }[]> {
	return unwrap(commands.coreGetTags());
}
