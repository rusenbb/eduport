import type { EntityDetail } from '$lib/types';

const ICON_KEYS = ['icon', 'emoji'] as const;
const COVER_KEYS = ['cover', 'banner'] as const;

function readCustomString(detail: EntityDetail | null | undefined, keys: readonly string[]): string {
	if (!detail) return '';
	const custom = (detail.entity as { custom?: Record<string, unknown> }).custom;
	if (!custom) return '';
	for (const k of keys) {
		const v = custom[k];
		if (typeof v === 'string' && v.trim()) return v.trim();
	}
	return '';
}

export function extractIcon(detail: EntityDetail | null | undefined): string {
	return readCustomString(detail, ICON_KEYS);
}

export function extractCover(detail: EntityDetail | null | undefined): string {
	return readCustomString(detail, COVER_KEYS);
}
