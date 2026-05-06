import { get } from 'svelte/store';
import { beforeEach, describe, expect, it } from 'vitest';
import { filters } from '../../src/lib/stores/filters';

describe('filters store', () => {
	beforeEach(() => {
		filters.clear();
	});

	it('addTag is idempotent', () => {
		filters.addTag('ai');
		filters.addTag('ai');
		expect(get(filters).tags).toEqual(['ai']);
	});

	it('removeTag drops only the named tag', () => {
		filters.addTag('ai');
		filters.addTag('theory');
		filters.removeTag('ai');
		expect(get(filters).tags).toEqual(['theory']);
	});

	it('toggleTag adds when absent and removes when present', () => {
		filters.toggleTag('ai');
		expect(get(filters).tags).toEqual(['ai']);
		filters.toggleTag('ai');
		expect(get(filters).tags).toEqual([]);
	});

	it('clear resets to empty', () => {
		filters.addTag('ai');
		filters.setLevel('masters');
		filters.clear();
		expect(get(filters)).toEqual({ tags: [] });
	});
});
