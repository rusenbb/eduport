import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ApiError, apiFetch } from '../../src/lib/api/client';

describe('apiFetch', () => {
	const fetchMock = vi.fn();

	beforeEach(() => {
		vi.stubGlobal('fetch', fetchMock);
		vi.stubEnv('VITE_SIDECAR_URL', 'http://test.local:9999');
	});

	afterEach(() => {
		fetchMock.mockReset();
		vi.unstubAllEnvs();
		vi.unstubAllGlobals();
	});

	it('prepends base URL and parses JSON on success', async () => {
		fetchMock.mockResolvedValueOnce(
			new Response(JSON.stringify({ ok: 1 }), {
				status: 200,
				headers: { 'Content-Type': 'application/json' }
			})
		);
		const result = await apiFetch('/health');
		expect(fetchMock).toHaveBeenCalledWith(
			'http://test.local:9999/health',
			expect.any(Object)
		);
		expect(result).toEqual({ ok: 1 });
	});

	it('throws ApiError with detail message on 4xx', async () => {
		fetchMock.mockResolvedValueOnce(
			new Response(JSON.stringify({ detail: 'not found' }), {
				status: 404,
				headers: { 'Content-Type': 'application/json' }
			})
		);
		let caught: unknown = null;
		try {
			await apiFetch('/missing');
		} catch (err) {
			caught = err;
		}
		expect(caught).toBeInstanceOf(ApiError);
		expect((caught as ApiError).status).toBe(404);
		expect((caught as ApiError).message).toBe('not found');
	});

	it('returns undefined on 204', async () => {
		fetchMock.mockResolvedValueOnce(new Response(null, { status: 204 }));
		const result = await apiFetch('/something', { method: 'DELETE' });
		expect(result).toBeUndefined();
	});
});
