import { commands } from '../bindings';
import type { AppStatus, ParseErrorItem } from '../types';
import { unwrap } from './client';

export function getAppStatus(): Promise<AppStatus> {
	return unwrap(commands.coreGetStatus()) as Promise<AppStatus>;
}

export function listParseErrors(): Promise<ParseErrorItem[]> {
	return unwrap(commands.coreListParseErrors());
}
