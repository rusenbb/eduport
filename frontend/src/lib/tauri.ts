// Thin wrappers around Tauri APIs that no-op in the browser dev server.
// Detection follows Tauri 2's runtime: window.__TAURI_INTERNALS__ exists
// when running inside a Tauri WebView.

import { putSettings } from './api/settings';
import type { Settings } from './types';

declare global {
	interface Window {
		__TAURI_INTERNALS__?: unknown;
	}
}

export interface BootstrapStatus {
	settings_exists: boolean;
	settings_path: string;
	/** True once the Rust state has finished its boot reconcile. */
	core_ready: boolean;
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

export async function getBootstrapStatus(): Promise<BootstrapStatus | null> {
	if (!isTauri()) return null;
	const { invoke } = await import('@tauri-apps/api/core');
	return invoke<BootstrapStatus>('core_bootstrap_status');
}

/**
 * First-run setup: persist settings and boot the Rust eduport-core
 * state (which `core_settings_put` does atomically). Replaces the
 * old `bootstrap_settings` Tauri command, which used to spawn the
 * Python sidecar — gone since rewrite phase 11.
 */
export async function bootstrapSettings(settings: Settings): Promise<Settings> {
	return putSettings(settings);
}

export async function setAppZoom(zoomFactor: number): Promise<void> {
	if (!isTauri()) return;
	const { invoke } = await import('@tauri-apps/api/core');
	await invoke('set_app_zoom', { zoomFactor });
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

export async function openPath(path: string): Promise<void> {
	if (!isTauri()) {
		window.open(path, '_blank');
		return;
	}
	const { invoke } = await import('@tauri-apps/api/core');
	await invoke('open_path', { path });
}

export async function saveCopyAs(sourcePath: string, suggestedName: string): Promise<void> {
	if (!isTauri()) {
		alert(`(dev) Would save a copy of: ${sourcePath}`);
		return;
	}
	const { save } = await import('@tauri-apps/plugin-dialog');
	const { invoke } = await import('@tauri-apps/api/core');
	const destinationPath = await save({ defaultPath: suggestedName });
	if (typeof destinationPath !== 'string' || destinationPath.length === 0) return;
	await invoke('copy_file', { sourcePath, destinationPath });
}

export async function cloneFileToFolder(sourcePath: string, filename: string): Promise<void> {
	if (!isTauri()) {
		alert(`(dev) Would clone ${sourcePath} to a folder`);
		return;
	}
	const destinationFolder = await pickFolder();
	if (!destinationFolder) return;
	const cleanFolder = destinationFolder.replace(/\/$/, '');
	const destinationPath = `${cleanFolder}/${filename}`;
	const { invoke } = await import('@tauri-apps/api/core');
	await invoke('copy_file', { sourcePath, destinationPath });
}

export async function readFileBytes(path: string): Promise<Uint8Array> {
	if (!isTauri()) throw new Error('File path import is only available in the desktop app.');
	const { invoke } = await import('@tauri-apps/api/core');
	const bytes = await invoke<number[]>('read_file_bytes', { path });
	return new Uint8Array(bytes);
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

// window.confirm is a no-op in Tauri WebViews; route through plugin-dialog.
export async function confirmDestructive(message: string, title = 'Eduport'): Promise<boolean> {
	if (!isTauri()) return window.confirm(message);
	const { ask } = await import('@tauri-apps/plugin-dialog');
	return ask(message, { title, kind: 'warning' });
}
