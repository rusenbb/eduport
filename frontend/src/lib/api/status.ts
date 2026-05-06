import { apiFetch } from './client';
import type { AppStatus, ParseErrorItem } from '../types';

export function getAppStatus(): Promise<AppStatus> {
	return apiFetch('/status');
}

export function listParseErrors(): Promise<ParseErrorItem[]> {
	return apiFetch('/parse-errors');
}
