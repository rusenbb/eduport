<script lang="ts">
	import { updateEntity } from '$lib/api/entities';
	import type { EntityDetail } from '$lib/types';
	import BodyEditor from './BodyEditor.svelte';

	let {
		detail,
		onCancel,
		onDone
	}: {
		detail: EntityDetail;
		onCancel?: () => void;
		onDone?: () => void;
	} = $props();

	let body = $state('');
	let saving = $state(false);
	let error: string | null = $state(null);
	let initialized = $state(false);

	$effect(() => {
		if (initialized) return;
		body = detail.body;
		initialized = true;
	});

	async function save() {
		error = null;
		saving = true;
		try {
			await updateEntity(detail.type, detail.file_id, detail.entity, body);
			onDone?.();
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			saving = false;
		}
	}
</script>

<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
	<div class="flex h-[82vh] w-[820px] max-w-[94vw] flex-col overflow-hidden rounded-lg border border-[var(--color-border)] bg-[var(--color-panel)] shadow-2xl">
		<header class="border-b border-[var(--color-border)] px-5 py-3">
			<h2 class="text-lg font-semibold">Edit body</h2>
			<p class="mt-1 text-xs text-[var(--color-muted)]">{detail.entity.name as string}</p>
		</header>

		<div class="min-h-0 flex-1 overflow-auto px-5 py-4">
			<BodyEditor bind:value={body} />
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
				disabled={saving}
				onclick={save}
			>
				{saving ? 'Saving...' : 'Save body'}
			</button>
		</footer>
	</div>
</div>
