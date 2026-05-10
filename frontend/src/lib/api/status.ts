import { coreInvoke } from './client';
import type { AppStatus, ParseErrorItem } from '../types';

export function getAppStatus(): Promise<AppStatus> {
	return coreInvoke('core_get_status');
}

export function listParseErrors(): Promise<ParseErrorItem[]> {
	return coreInvoke('core_list_parse_errors');
}
