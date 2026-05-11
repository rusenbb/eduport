// Single source of truth for the API transport.
//
// Every `api/*.ts` module dispatches through the typed `commands`
// namespace generated from the Rust side by tauri-specta (see
// `frontend/src/lib/bindings.ts` and the Rust `lib.rs` test that
// regenerates it). The transport itself goes through the Tauri
// command channel; outside Tauri (the SvelteKit dev server,
// vitest), the underlying `invoke` throws — the dev server is
// expected to point at a Tauri build for any test that exercises
// the API.

import { listen as tauriListen, type UnlistenFn } from '@tauri-apps/api/event';

import type { CommandError, Result } from '../bindings';

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
 * Unwrap a `Result<T, E>` returned by a tauri-specta-generated
 * command. On `status: "ok"` returns the data; on `status: "error"`
 * throws a [`CoreCommandError`] carrying the structured `code` so
 * callers can branch on the error class without parsing prose.
 *
 * `E` is typically `CommandError` (the Rust-side struct that derives
 * `specta::Type`) but a handful of host-shell commands return
 * `string` instead — that branch folds into an `internal`-coded
 * `CoreCommandError` so the rest of the frontend sees one error
 * type everywhere.
 */
export async function unwrap<T, E = CommandError>(
	call: Promise<Result<T, E>>
): Promise<T> {
	const result = await call;
	if (result.status === 'ok') {
		return result.data;
	}
	const err = result.error;
	if (err && typeof err === 'object' && 'code' in err && 'message' in err) {
		throw new CoreCommandError(err as CommandErrorPayload);
	}
	throw new CoreCommandError({
		code: 'internal',
		message: typeof err === 'string' ? err : String(err)
	});
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
