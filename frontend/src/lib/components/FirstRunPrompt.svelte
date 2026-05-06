<script lang="ts">
	import { putSettings } from '$lib/api/settings';
	import { settings } from '$lib/stores/settings';
	import { pickFolder } from '$lib/tauri';
	import type { Settings } from '$lib/types';

	let dataFolder = $state('');
	let userEmail = $state('');
	let saving = $state(false);
	let error: string | null = $state(null);

	async function browse() {
		const picked = await pickFolder();
		if (picked) dataFolder = picked;
	}

	async function save() {
		error = null;
		if (!dataFolder.trim() || !userEmail.trim()) {
			error = 'Both data folder and email are required.';
			return;
		}
		const payload: Settings = {
			data_folder: dataFolder.trim(),
			attachments_folder: './attachments',
			notes_folder: './notes',
			theme: 'system',
			user_email: userEmail.trim()
		};
		saving = true;
		try {
			const result = await putSettings(payload);
			settings.set(result);
			location.reload();
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			saving = false;
		}
	}
</script>

<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/80">
	<div class="flex w-[480px] max-w-[90vw] flex-col rounded-lg border border-[var(--color-border)] bg-[var(--color-panel)] p-6 shadow-2xl">
		<h2 class="text-xl font-semibold">Welcome to Eduport</h2>
		<p class="mt-1 text-sm text-[var(--color-muted)]">
			Pick a data folder. Eduport will create <code>attachments/</code> and <code>notes/</code> subfolders
			inside it. Your data stays on your machine.
		</p>

		<label class="mt-5 block">
			<span class="block text-xs uppercase tracking-wider text-[var(--color-muted)]">Data folder</span>
			<div class="mt-1 flex gap-2">
				<input
					bind:value={dataFolder}
					placeholder="/Users/you/Documents/Eduport"
					class="flex-1 rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-3 py-2 text-sm outline-none focus:border-[var(--color-accent)]"
				/>
				<button
					type="button"
					class="rounded border border-[var(--color-border)] bg-white/5 px-3 py-2 text-xs hover:bg-white/10"
					onclick={browse}
				>
					Browse…
				</button>
			</div>
		</label>

		<label class="mt-3 block">
			<span class="block text-xs uppercase tracking-wider text-[var(--color-muted)]">Your email (for inbound/outbound on .eml import)</span>
			<input
				bind:value={userEmail}
				type="email"
				placeholder="you@example.com"
				class="mt-1 w-full rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-3 py-2 text-sm outline-none focus:border-[var(--color-accent)]"
			/>
		</label>

		{#if error}
			<div class="mt-3 rounded border border-red-900 bg-red-900/30 p-2 text-xs text-[var(--color-bad)]">{error}</div>
		{/if}

		<button
			class="mt-5 rounded border border-blue-700 bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
			onclick={save}
			disabled={saving}
		>
			{saving ? 'Saving…' : 'Save and continue'}
		</button>
	</div>
</div>
