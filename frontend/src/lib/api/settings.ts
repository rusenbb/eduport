import { coreInvoke } from './client';
import type { Settings } from '../types';

export function getSettings(): Promise<Settings> {
	return coreInvoke('core_settings_get');
}

export function putSettings(settings: Settings): Promise<Settings> {
	return coreInvoke('core_settings_put', { settings });
}
