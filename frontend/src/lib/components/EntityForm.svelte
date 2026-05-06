<script lang="ts">
	import { goto } from '$app/navigation';
	import { createEntity, updateEntity } from '$lib/api/entities';
	import type { EntityType } from '$lib/types';
	import TagPicker from './TagPicker.svelte';

	let {
		type,
		fileId = null,
		initial = null,
		onCancel,
		onDone
	}: {
		type: EntityType;
		fileId?: string | null;
		initial?: { frontmatter: Record<string, unknown>; body: string } | null;
		onCancel?: () => void;
		onDone?: (fileId: string) => void;
	} = $props();

	const typeTag = `eduport-type/${type}`;

	function pickInitial(): {
		name: string;
		tags: string[];
		body: string;
		extras: string;
	} {
		const fm = initial?.frontmatter ?? {};
		const tags = Array.isArray(fm.tags)
			? (fm.tags as string[]).filter((t) => t !== typeTag)
			: [];
		const { name, tags: _t, ...rest } = fm as Record<string, unknown> & { name?: string; tags?: unknown };
		return {
			name: typeof name === 'string' ? name : '',
			tags,
			body: initial?.body ?? '',
			extras: Object.keys(rest).length === 0 ? '' : JSON.stringify(rest, null, 2)
		};
	}

	const initialForm = pickInitial();
	let name = $state(initialForm.name);
	let tags = $state<string[]>(initialForm.tags);
	let body = $state(initialForm.body);
	let extras = $state(initialForm.extras);
	let saving = $state(false);
	let error: string | null = $state(null);

	async function save() {
		error = null;

		let extrasObj: Record<string, unknown> = {};
		if (extras.trim().length > 0) {
			try {
				extrasObj = JSON.parse(extras);
				if (typeof extrasObj !== 'object' || extrasObj === null || Array.isArray(extrasObj)) {
					throw new Error('Extras must be a JSON object');
				}
			} catch (e) {
				error = `Extras JSON invalid: ${e instanceof Error ? e.message : String(e)}`;
				return;
			}
		}

		const frontmatter: Record<string, unknown> = {
			tags: [typeTag, ...tags],
			name,
			...extrasObj
		};

		saving = true;
		try {
			let resultId: string;
			if (fileId) {
				const r = await updateEntity(type, fileId, frontmatter, body);
				resultId = r.file_id;
			} else {
				const r = await createEntity(type, frontmatter, body);
				resultId = r.file_id;
			}
			onDone?.(resultId);
			goto(`/${type}/${resultId}`);
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			saving = false;
		}
	}
</script>

<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
	<div class="flex w-[640px] max-w-[90vw] max-h-[90vh] flex-col overflow-hidden rounded-lg border border-[var(--color-border)] bg-[var(--color-panel)] shadow-2xl">
		<header class="border-b border-[var(--color-border)] px-5 py-3">
			<h2 class="text-lg font-semibold">
				{fileId ? 'Edit' : 'New'} {type}
			</h2>
		</header>

		<div class="flex-1 overflow-auto px-5 py-4">
			<label class="block">
				<span class="block text-xs uppercase tracking-wider text-[var(--color-muted)]">Name</span>
				<input
					bind:value={name}
					class="mt-1 w-full rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-3 py-2 text-sm outline-none focus:border-[var(--color-accent)]"
					placeholder="e.g. ETH Zurich"
					required
				/>
			</label>

			<div class="mt-4">
				<span class="block text-xs uppercase tracking-wider text-[var(--color-muted)]">Tags</span>
				<div class="mt-1"><TagPicker bind:tags /></div>
			</div>

			<label class="mt-4 block">
				<span class="block text-xs uppercase tracking-wider text-[var(--color-muted)]">
					Other fields (JSON, optional)
				</span>
				<textarea
					bind:value={extras}
					class="mt-1 h-32 w-full rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-3 py-2 font-mono text-xs outline-none focus:border-[var(--color-accent)]"
					placeholder={'{\n  "level": "masters",\n  "deadline": "2026-12-15"\n}'}
				></textarea>
			</label>

			<label class="mt-4 block">
				<span class="block text-xs uppercase tracking-wider text-[var(--color-muted)]">Body (markdown)</span>
				<textarea
					bind:value={body}
					class="mt-1 h-40 w-full rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-3 py-2 font-mono text-sm outline-none focus:border-[var(--color-accent)]"
				></textarea>
			</label>

			{#if error}
				<div class="mt-3 rounded border border-red-900 bg-red-900/30 p-2 text-xs text-[var(--color-bad)]">{error}</div>
			{/if}
		</div>

		<footer class="flex items-center justify-end gap-2 border-t border-[var(--color-border)] px-5 py-3">
			<button class="rounded border border-[var(--color-border)] bg-white/5 px-3 py-1.5 text-xs hover:bg-white/10" onclick={onCancel}>
				Cancel
			</button>
			<button
				class="rounded border border-blue-700 bg-blue-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-blue-700 disabled:opacity-50"
				disabled={!name.trim() || saving}
				onclick={save}
			>
				{saving ? 'Saving…' : 'Save'}
			</button>
		</footer>
	</div>
</div>
