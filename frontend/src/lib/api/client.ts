// Single source of truth for the API transport.
//
// Phase-10 cutover: every `api/*.ts` module now calls `coreInvoke()`
// (Tauri command channel) by default. The legacy `apiFetch()` is
// retained for the bootstrap flow (`/get_bootstrap_status`,
// `/ensure_sidecar_started`) only — those still talk to the
// Tauri-managed sidecar handle, which the frontend keeps using
// during the Phase-10 transition. Phase 11 deletes both the
// sidecar and the `apiFetch()` path.
//
// Outside Tauri (the SvelteKit dev server, vitest), `coreInvoke`
// throws — the dev server is expected to point at a running
// sidecar or run pure-frontend tests that don't hit the API.

import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import { listen as tauriListen, type UnlistenFn } from '@tauri-apps/api/event';

declare global {
	interface Window {
		__EDUPORT_API_URL__?: string;
		__TAURI_INTERNALS__?: unknown;
	}
}

/** Stable error envelope returned by every `core_*` command. */
export interface CommandErrorPayload {
	code: string;
	message: string;
}

/**
 * Thrown when a Tauri command rejects. Carries the structured
 * `code` so callers can branch on the error class without parsing
 * prose. The `code` set is stable as part of the API contract:
 * `invalid`, `not_found`, `conflict`, `internal`, `not_initialised`.
 */
export class CoreCommandError extends Error {
	public readonly code: string;

	constructor(payload: CommandErrorPayload) {
		super(payload.message);
		this.name = 'CoreCommandError';
		this.code = payload.code;
	}
}

/** Legacy ApiError, kept for the sidecar bootstrap flow. */
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

/** Returns true when running inside the Tauri shell. */
export function isTauri(): boolean {
	return typeof window !== 'undefined' && !!window.__TAURI_INTERNALS__;
}

/**
 * Invoke a Tauri command exposed by `eduport-tauri`. Wraps the raw
 * `invoke` call so callers see a typed return value plus a typed
 * error class.
 *
 * Tauri sends Rust panics back as plain strings; structured errors
 * come back as JSON of `CommandErrorPayload`. We branch on the
 * shape so panic messages still surface usefully without a
 * `code` field crash.
 */
export async function coreInvoke<T = unknown>(
	command: string,
	args: Record<string, unknown> = {}
): Promise<T> {
	try {
		return (await tauriInvoke(command, args)) as T;
	} catch (raw) {
		if (raw && typeof raw === 'object' && 'code' in raw && 'message' in raw) {
			throw new CoreCommandError(raw as CommandErrorPayload);
		}
		// String / unknown shape: fold into an internal-coded
		// CommandError so all rejections share one type.
		throw new CoreCommandError({
			code: 'internal',
			message: typeof raw === 'string' ? raw : String(raw)
		});
	}
}

/**
 * Subscribe to a Tauri event emitted by `eduport-tauri`. Two
 * channels are public:
 * - `eduport:vault-event` — typed VaultEvent JSON (entity_changed,
 *   entity_deleted, schema_changed, views_changed, needs_rescan).
 * - `eduport:parse-error` — `{ path, message }` when the watcher
 *   sees a file whose frontmatter doesn't parse.
 *
 * Returns the unlisten function so callers can clean up on
 * component teardown.
 */
export async function listenCoreEvent<T = unknown>(
	event: string,
	handler: (payload: T) => void
): Promise<UnlistenFn> {
	return tauriListen<T>(event, (e) => handler(e.payload as T));
}

// ── Legacy sidecar transport (bootstrap path only) ───────────────

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
