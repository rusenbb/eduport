import { writable } from 'svelte/store';

export interface FilterState {
	tags: string[];
	level?: string;
	country?: string;
}

function createFiltersStore() {
	const { subscribe, update, set } = writable<FilterState>({ tags: [] });

	return {
		subscribe,
		set,
		addTag(tag: string) {
			update((s) => (s.tags.includes(tag) ? s : { ...s, tags: [...s.tags, tag] }));
		},
		removeTag(tag: string) {
			update((s) => ({ ...s, tags: s.tags.filter((t) => t !== tag) }));
		},
		toggleTag(tag: string) {
			update((s) =>
				s.tags.includes(tag)
					? { ...s, tags: s.tags.filter((t) => t !== tag) }
					: { ...s, tags: [...s.tags, tag] }
			);
		},
		setLevel(level: string | undefined) {
			update((s) => ({ ...s, level }));
		},
		setCountry(country: string | undefined) {
			update((s) => ({ ...s, country }));
		},
		clear() {
			set({ tags: [] });
		}
	};
}

export const filters = createFiltersStore();
