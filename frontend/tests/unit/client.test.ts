import { describe, expect, it } from 'vitest';
import { CoreCommandError, unwrap } from '../../src/lib/api/client';
import type { Result } from '../../src/lib/bindings';

// The legacy `coreInvoke` (string-based command dispatch) was
// replaced by tauri-specta-generated `commands` — see
// `frontend/src/lib/bindings.ts`. The `unwrap` helper converts the
// generated `Promise<Result<T, E>>` shape into the throw-on-error
// model the rest of the frontend already expects.

describe('unwrap', () => {
	it('returns the data when the result is ok', async () => {
		const result: Result<{ ok: number }, never> = { status: 'ok', data: { ok: 1 } };
		await expect(unwrap(Promise.resolve(result))).resolves.toEqual({ ok: 1 });
	});

	it('wraps a structured CommandError preserving the code', async () => {
		const result: Result<unknown, { code: string; message: string }> = {
			status: 'error',
			error: { code: 'not_found', message: 'no such entity' }
		};
		try {
			await unwrap(Promise.resolve(result));
			throw new Error('expected unwrap to reject');
		} catch (err) {
			expect(err).toBeInstanceOf(CoreCommandError);
			expect((err as CoreCommandError).code).toBe('not_found');
			expect((err as CoreCommandError).message).toBe('no such entity');
		}
	});

	it('folds bare-string errors (host-shell commands) into the internal code', async () => {
		const result: Result<unknown, string> = { status: 'error', error: 'something exploded' };
		try {
			await unwrap(Promise.resolve(result));
			throw new Error('expected unwrap to reject');
		} catch (err) {
			expect(err).toBeInstanceOf(CoreCommandError);
			expect((err as CoreCommandError).code).toBe('internal');
			expect((err as CoreCommandError).message).toBe('something exploded');
		}
	});
});
