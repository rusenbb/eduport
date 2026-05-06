// SPA mode: render entirely client-side. Tauri loads index.html and the
// sidecar URL is injected at runtime via window.__EDUPORT_API_URL__.
export const ssr = false;
export const prerender = false;

import { getSettings } from '$lib/api/settings';
import { settings } from '$lib/stores/settings';
import { status } from '$lib/stores/status';
import type { LayoutLoad } from './$types';

export const load: LayoutLoad = async () => {
	status.startPolling();
	try {
		const s = await getSettings();
		settings.set(s);
		return { hasSettings: true };
	} catch {
		// Sidecar is down or has no settings yet — first-run flow takes over.
		return { hasSettings: false };
	}
};
