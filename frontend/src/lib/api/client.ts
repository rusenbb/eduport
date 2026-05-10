// Single source of truth for the API transport.
//
// Every `api/*.ts` module routes through `coreInvoke()` (Tauri
// command channel). The HTTP-fetch path that used to talk to the
// Python sidecar was removed in rewrite phase 11; outside Tauri
// (the SvelteKit dev server, vitest), `coreInvoke` throws — the
// dev server is expected to point at a Tauri build for any test
// that exercises the API.

import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import { listen as tauriListen, type UnlistenFn } from '@tauri-apps/api/event';

declare global {
	interface Window {
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
