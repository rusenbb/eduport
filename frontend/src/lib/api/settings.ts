import { commands, type SettingsDto } from '../bindings';
import type { Settings } from '../types';
import { unwrap } from './client';

export function getSettings(): Promise<Settings> {
	return unwrap(commands.coreSettingsGet()) as Promise<Settings>;
}

export function putSettings(settings: Settings): Promise<Settings> {
	return unwrap(commands.coreSettingsPut(settings as unknown as SettingsDto)) as Promise<Settings>;
}
