import { describe, expect, it, vi } from 'vitest';
import { CoreCommandError, coreInvoke } from '../../src/lib/api/client';

// The legacy `apiFetch` (HTTP transport to the Python sidecar) was
// removed in rewrite phase 11. The `coreInvoke` helper is now the
// only transport. The tests here exercise its error-folding
// behaviour against a stubbed `@tauri-apps/api/core::invoke`.

vi.mock('@tauri-apps/api/core', () => ({
	invoke: vi.fn()
}));

describe('coreInvoke', () => {
	it('returns the resolved value when invoke succeeds', async () => {
		const mod = await import('@tauri-apps/api/core');
		(mod.invoke as ReturnType<typeof vi.fn>).mockResolvedValueOnce({ ok: 1 });
		await expect(coreInvoke('core_get_status')).resolves.toEqual({ ok: 1 });
	});

	it('wraps a structured error into CoreCommandError preserving the code', async () => {
		const mod = await import('@tauri-apps/api/core');
		(mod.invoke as ReturnType<typeof vi.fn>).mockRejectedValueOnce({
			code: 'not_found',
			message: 'no such entity'
		});
		try {
			await coreInvoke('core_entity_get');
			throw new Error('expected coreInvoke to reject');
		} catch (err) {
			expect(err).toBeInstanceOf(CoreCommandError);
			expect((err as CoreCommandError).code).toBe('not_found');
			expect((err as CoreCommandError).message).toBe('no such entity');
		}
	});

	it('folds bare-string errors (Rust panic surfaces) into the internal code', async () => {
		const mod = await import('@tauri-apps/api/core');
		(mod.invoke as ReturnType<typeof vi.fn>).mockRejectedValueOnce('something exploded');
		try {
			await coreInvoke('core_search');
			throw new Error('expected coreInvoke to reject');
		} catch (err) {
			expect(err).toBeInstanceOf(CoreCommandError);
			expect((err as CoreCommandError).code).toBe('internal');
			expect((err as CoreCommandError).message).toBe('something exploded');
		}
	});
});
