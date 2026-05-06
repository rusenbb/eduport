// Single source of truth for the sidecar URL.
// In Tauri, the shell injects window.__EDUPORT_API_URL__ at runtime.
// In the dev server (no Tauri), VITE_SIDECAR_URL is read from .env.
declare global {
	interface Window {
		__EDUPORT_API_URL__?: string;
	}
}

export class ApiError extends Error {
	constructor(
		public status: number,
		public detail: unknown,
		message: string
	) {
		super(message);
		this.name = 'ApiError';
	}
}

function baseUrl(): string {
	if (typeof window !== 'undefined' && window.__EDUPORT_API_URL__) {
		return window.__EDUPORT_API_URL__;
	}
	return import.meta.env.VITE_SIDECAR_URL ?? 'http://127.0.0.1:8765';
}

export async function apiFetch<T = unknown>(
	path: string,
	init: RequestInit = {}
): Promise<T> {
	const url = `${baseUrl().replace(/\/$/, '')}${path.startsWith('/') ? path : `/${path}`}`;
	const headers = new Headers(init.headers);
	if (init.body && !headers.has('Content-Type') && !(init.body instanceof FormData)) {
		headers.set('Content-Type', 'application/json');
	}
	const res = await fetch(url, { ...init, headers });
	if (!res.ok) {
		let detail: unknown = null;
		try {
			detail = await res.json();
		} catch {
			detail = await res.text().catch(() => null);
		}
		const detailMsg =
			(detail && typeof detail === 'object' && 'detail' in detail
				? (detail as { detail: unknown }).detail
				: detail) ?? `HTTP ${res.status}`;
		throw new ApiError(res.status, detail, String(detailMsg));
	}
	if (res.status === 204) return undefined as T;
	const ct = res.headers.get('Content-Type') ?? '';
	if (ct.includes('application/json')) return (await res.json()) as T;
	return (await res.text()) as T;
}
