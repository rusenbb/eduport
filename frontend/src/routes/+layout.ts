// SPA mode: render entirely client-side. Tauri loads index.html and the
// sidecar URL is injected at runtime via window.__EDUPORT_API_URL__.
export const ssr = false;
export const prerender = false;

import { getSettings } from '$lib/api/settings';
import { settings } from '$lib/stores/settings';
import { status } from '$lib/stores/status';
import { ensureSidecarUrl, getBootstrapStatus } from '$lib/tauri';
import type { LayoutLoad } from './$types';

export const load: LayoutLoad = async () => {
	try {
		const bootstrap = await getBootstrapStatus();
		if (bootstrap && !bootstrap.settings_exists) {
			return { hasSettings: false };
		}
		await ensureSidecarUrl();
		status.startPolling();
	} catch {
		status.startPolling();
		// If the shell cannot start the sidecar, the status banner handles it below.
	}
	try {
		const s = await getSettings();
		settings.set(s);
		return { hasSettings: true };
	} catch {
		// Sidecar is down or has no settings yet — first-run flow takes over.
		return { hasSettings: false };
	}
};
