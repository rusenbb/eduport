// SPA mode: Tauri loads index.html and the Rust eduport-core state
// boots before the WebView is ready. There is no longer a separate
// sidecar URL to inject — the Tauri command channel is the only
// transport.
export const ssr = false;
export const prerender = false;

import { getSettings } from '$lib/api/settings';
import { settings } from '$lib/stores/settings';
import { status } from '$lib/stores/status';
import { getBootstrapStatus } from '$lib/tauri';
import type { LayoutLoad } from './$types';

export const load: LayoutLoad = async () => {
	try {
		const bootstrap = await getBootstrapStatus();
		if (bootstrap && !bootstrap.settings_exists) {
			return { hasSettings: false };
		}
		status.startPolling();
	} catch {
		status.startPolling();
		// If the Rust state cannot boot, the status banner handles it below.
	}
	try {
		const s = await getSettings();
		settings.set(s);
		return { hasSettings: true };
	} catch {
		// eduport-core not ready yet (no settings), so first-run flow takes over.
		return { hasSettings: false };
	}
};
