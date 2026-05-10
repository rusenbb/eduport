import { writable } from 'svelte/store';
import { CoreCommandError } from '../api/client';
import { getAppStatus } from '../api/status';

export interface StatusState {
	/** True when the Rust eduport-core state is reachable.
	 *  Renamed from `sidecarUp` after rewrite phase 11; the field
	 *  signals the same thing — "the backend is healthy". */
	coreUp: boolean;
	parseErrors: number;
	lastChecked: number;
}

function createStatusStore() {
	const { subscribe, set } = writable<StatusState>({
		coreUp: false,
		parseErrors: 0,
		lastChecked: 0
	});

	let timer: ReturnType<typeof setInterval> | null = null;

	async function check() {
		try {
			const appStatus = await getAppStatus();
			set({ coreUp: true, parseErrors: appStatus.parse_errors, lastChecked: Date.now() });
		} catch (err) {
			// `not_initialised` means the Rust state hasn't loaded yet
			// (no settings file, or settings_put hasn't happened) — the
			// status banner treats that the same as "down".
			const down =
				err instanceof CoreCommandError && err.code === 'not_initialised';
			set({
				coreUp: !down,
				parseErrors: 0,
				lastChecked: Date.now()
			});
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
