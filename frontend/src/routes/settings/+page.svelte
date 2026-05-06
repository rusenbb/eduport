<script lang="ts">
	import { putSettings } from '$lib/api/settings';
	import { settings } from '$lib/stores/settings';
	import { pickFolder, setAppZoom } from '$lib/tauri';
	import type { Settings } from '$lib/types';

	const zoomOptions = [
		{ label: '90%', value: 0.9 },
		{ label: '100%', value: 1 },
		{ label: '110%', value: 1.1 },
		{ label: '125%', value: 1.25 },
		{ label: '150%', value: 1.5 }
	];

	let draft: Settings = $state({
		data_folder: '',
		attachments_folder: './attachments',
		notes_folder: './notes',
		theme: 'system',
		user_email: '',
		zoom_factor: 1,
		obsidian_vault: null,
		confirm_deletes: true
	});
	let loadedFromStore = $state(false);
	let saving = $state(false);
	let saved = $state(false);
	let error: string | null = $state(null);

	$effect(() => {
		if ($settings && !loadedFromStore) {
			draft = {
				...$settings,
				zoom_factor: $settings.zoom_factor ?? 1,
				obsidian_vault: $settings.obsidian_vault ?? null,
				confirm_deletes: $settings.confirm_deletes ?? true
			};
			loadedFromStore = true;
		}
	});

	async function browse() {
		const folder = await pickFolder();
		if (folder) draft = { ...draft, data_folder: folder };
	}

	async function save() {
		error = null;
		saved = false;
		saving = true;
		try {
			const payload: Settings = {
				...draft,
				zoom_factor: Number(draft.zoom_factor),
				obsidian_vault: draft.obsidian_vault?.trim() || null,
				confirm_deletes: Boolean(draft.confirm_deletes)
			};
			const result = await putSettings(payload);
			settings.set(result);
			await setAppZoom(result.zoom_factor ?? 1);
			saved = true;
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			saving = false;
		}
	}
</script>

<main class="mx-auto flex max-w-3xl flex-col gap-6 p-6">
	<header>
		<h1 class="text-2xl font-semibold">Settings</h1>
		<p class="mt-1 text-sm text-[var(--color-muted)]">Paths, import identity, integrations, and desktop zoom.</p>
	</header>

	<section class="grid gap-4 border-y border-[var(--color-border)] py-5">
		<label class="grid gap-1">
			<span class="text-xs uppercase tracking-wider text-[var(--color-muted)]">Data folder</span>
			<div class="flex gap-2">
				<input
					bind:value={draft.data_folder}
					class="min-w-0 flex-1 rounded border border-[var(--color-border)] bg-[var(--color-panel)] px-3 py-2 text-sm outline-none focus:border-[var(--color-accent)]"
				/>
				<button class="rounded border border-[var(--color-border)] px-3 py-2 text-xs hover:bg-white/5" onclick={browse}>
					Browse
				</button>
			</div>
		</label>

		<div class="grid gap-4 md:grid-cols-2">
			<label class="grid gap-1">
				<span class="text-xs uppercase tracking-wider text-[var(--color-muted)]">Attachments folder</span>
				<input
					bind:value={draft.attachments_folder}
					class="rounded border border-[var(--color-border)] bg-[var(--color-panel)] px-3 py-2 text-sm outline-none focus:border-[var(--color-accent)]"
				/>
			</label>
			<label class="grid gap-1">
				<span class="text-xs uppercase tracking-wider text-[var(--color-muted)]">Notes folder</span>
				<input
					bind:value={draft.notes_folder}
					class="rounded border border-[var(--color-border)] bg-[var(--color-panel)] px-3 py-2 text-sm outline-none focus:border-[var(--color-accent)]"
				/>
			</label>
		</div>

		<div class="grid gap-4 md:grid-cols-2">
			<label class="grid gap-1">
				<span class="text-xs uppercase tracking-wider text-[var(--color-muted)]">Your email</span>
				<input
					bind:value={draft.user_email}
					type="email"
					class="rounded border border-[var(--color-border)] bg-[var(--color-panel)] px-3 py-2 text-sm outline-none focus:border-[var(--color-accent)]"
				/>
			</label>
			<label class="grid gap-1">
				<span class="text-xs uppercase tracking-wider text-[var(--color-muted)]">Obsidian vault</span>
				<input
					value={draft.obsidian_vault ?? ''}
					oninput={(event) => (draft = { ...draft, obsidian_vault: event.currentTarget.value })}
					placeholder="Optional"
					class="rounded border border-[var(--color-border)] bg-[var(--color-panel)] px-3 py-2 text-sm outline-none focus:border-[var(--color-accent)]"
				/>
			</label>
		</div>
	</section>

	<section class="grid gap-4 border-b border-[var(--color-border)] pb-5">
		<div class="grid gap-4 md:grid-cols-2">
			<label class="grid gap-1">
				<span class="text-xs uppercase tracking-wider text-[var(--color-muted)]">Theme</span>
				<select
					bind:value={draft.theme}
					class="rounded border border-[var(--color-border)] bg-[var(--color-panel)] px-3 py-2 text-sm text-[var(--color-text)] outline-none focus:border-[var(--color-accent)]"
				>
					<option value="system">System</option>
					<option value="dark">Dark</option>
					<option value="light">Light</option>
				</select>
			</label>
			<label class="grid gap-1">
				<span class="text-xs uppercase tracking-wider text-[var(--color-muted)]">Zoom</span>
				<select
					value={String(draft.zoom_factor)}
					onchange={(event) => (draft = { ...draft, zoom_factor: Number(event.currentTarget.value) })}
					class="rounded border border-[var(--color-border)] bg-[var(--color-panel)] px-3 py-2 text-sm text-[var(--color-text)] outline-none focus:border-[var(--color-accent)]"
				>
					{#each zoomOptions as option}
						<option value={String(option.value)}>{option.label}</option>
					{/each}
				</select>
			</label>
		</div>
		<label class="flex items-center gap-3 text-sm">
			<input
				type="checkbox"
				checked={!draft.confirm_deletes}
				onchange={(event) => (draft = { ...draft, confirm_deletes: !event.currentTarget.checked })}
				class="accent-[var(--color-accent)]"
			/>
			<span>Do not ask for confirmation when deleting</span>
		</label>
	</section>

	<footer class="flex items-center gap-3">
		<button
			class="rounded border border-blue-700 bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
			disabled={saving || !draft.data_folder.trim() || !draft.user_email.trim()}
			onclick={save}
		>
			{saving ? 'Saving...' : 'Save settings'}
		</button>
		{#if saved}
			<span class="text-sm text-[var(--color-good)]">Saved</span>
		{/if}
		{#if error}
			<span class="text-sm text-[var(--color-bad)]">{error}</span>
		{/if}
	</footer>
</main>
