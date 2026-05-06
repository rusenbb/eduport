import { writable } from 'svelte/store';
import { apiFetch, ApiError } from '../api/client';

export interface StatusState {
	sidecarUp: boolean;
	parseErrors: number;
	lastChecked: number;
}

function createStatusStore() {
	const { subscribe, set } = writable<StatusState>({
		sidecarUp: false,
		parseErrors: 0,
		lastChecked: 0
	});

	let timer: ReturnType<typeof setInterval> | null = null;

	async function check() {
		try {
			await apiFetch('/health');
			set({ sidecarUp: true, parseErrors: 0, lastChecked: Date.now() });
		} catch (err) {
			set({
				sidecarUp: !(err instanceof ApiError && err.status === 0),
				parseErrors: 0,
				lastChecked: Date.now()
			});
			if (err instanceof TypeError) {
				// network failure — fetch threw before getting a response
				set({ sidecarUp: false, parseErrors: 0, lastChecked: Date.now() });
			}
		}
	}

	return {
		subscribe,
		check,
		startPolling(intervalMs = 10_000) {
			if (timer !== null) return;
			void check();
			timer = setInterval(() => void check(), intervalMs);
		},
		stopPolling() {
			if (timer !== null) {
				clearInterval(timer);
				timer = null;
			}
		}
	};
}

export const status = createStatusStore();
