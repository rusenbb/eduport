// Lightweight non-blocking notification stack. Used in place of
// `window.alert()` for transient errors, save confirmations, and other
// background-event signals where blocking the UI would be annoying.
//
// API:
//   toasts.success(msg, detail?)  → green, 4s
//   toasts.error(msg, detail?)    → red, 8s (errors deserve time to read)
//   toasts.info(msg, detail?)     → neutral, 4s
//   toasts.dismiss(id)            → drop early
//
// Mount `<ToastHost />` once (top-level layout) and the rest of the app
// gets a feedback channel for free.

import { writable } from 'svelte/store';

export type ToastKind = 'success' | 'error' | 'info';

export interface Toast {
	id: number;
	kind: ToastKind;
	message: string;
	detail?: string;
}

function createToastsStore() {
	const { subscribe, update } = writable<Toast[]>([]);
	let nextId = 1;

	function push(toast: Omit<Toast, 'id'>): number {
		const id = nextId++;
		const full: Toast = { id, ...toast };
		update((arr) => [...arr, full]);
		const ttl = toast.kind === 'error' ? 8000 : 4000;
		setTimeout(() => dismiss(id), ttl);
		return id;
	}

	function dismiss(id: number): void {
		update((arr) => arr.filter((t) => t.id !== id));
	}

	return {
		subscribe,
		success: (message: string, detail?: string) =>
			push({ kind: 'success', message, detail }),
		error: (message: string, detail?: string) =>
			push({ kind: 'error', message, detail }),
		info: (message: string, detail?: string) =>
			push({ kind: 'info', message, detail }),
		dismiss
	};
}

export const toasts = createToastsStore();
