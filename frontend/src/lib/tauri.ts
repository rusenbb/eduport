// Thin wrappers around Tauri APIs that no-op in the browser dev server.
// Detection follows Tauri 2's runtime: window.__TAURI_INTERNALS__ exists
// when running inside a Tauri WebView.

declare global {
	interface Window {
		__TAURI_INTERNALS__?: unknown;
	}
}

export function isTauri(): boolean {
	return typeof window !== 'undefined' && !!window.__TAURI_INTERNALS__;
}

export async function revealInFileManager(path: string): Promise<void> {
	if (!isTauri()) {
		alert(`(dev) Would reveal: ${path}`);
		return;
	}
	const { invoke } = await import('@tauri-apps/api/core');
	await invoke('reveal_in_file_manager', { path });
}

export async function openInObsidian(vault: string, file: string): Promise<void> {
	const url = `obsidian://open?vault=${encodeURIComponent(vault)}&file=${encodeURIComponent(file)}`;
	if (!isTauri()) {
		window.open(url, '_blank');
		return;
	}
	const { open } = await import('@tauri-apps/plugin-shell');
	await open(url);
}

export async function pickFolder(): Promise<string | null> {
	if (!isTauri()) {
		const typed = prompt('Pick a data folder (typed):');
		return typed?.trim() || null;
	}
	const { open } = await import('@tauri-apps/plugin-dialog');
	const result = await open({ directory: true, multiple: false });
	if (typeof result === 'string') return result;
	return null;
}
