import { writable } from 'svelte/store';
import type { Settings } from '../types';

export const settings = writable<Settings | null>(null);
