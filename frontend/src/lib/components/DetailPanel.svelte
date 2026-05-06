<script lang="ts">
	import { goto } from '$app/navigation';
	import type { EntityDetail } from '$lib/types';
	import DetailField from './DetailField.svelte';
	import BodyView from './BodyView.svelte';

	let {
		detail,
		onEditForm,
		onEditBody,
		onDelete
	}: {
		detail: EntityDetail;
		onEditForm?: () => void;
		onEditBody?: () => void;
		onDelete?: () => void;
	} = $props();

	const fields = $derived(
		Object.entries(detail.entity).filter(([k]) => k !== 'name' && k !== 'tags')
	);

	const tags = $derived(
		Array.isArray(detail.entity.tags)
			? (detail.entity.tags as string[]).filter(
					(t) => !t.startsWith('eduport-type/') && !t.startsWith('eduport-doctype/')
				)
			: []
	);

	function backTarget(srcFileId: string): string {
		// Best-effort: route to /university first; user can re-navigate. We don't have type
		// info on backlinks at the v1 API level. (The sidecar exposes `field` but not the
		// source type — that's a follow-up.)
		return `/university/${encodeURIComponent(srcFileId)}`;
	}
</script>

<div class="flex h-full w-full flex-col overflow-auto bg-[var(--color-panel)]">
	<header class="border-b border-[var(--color-border)] p-4">
		<h2 class="text-lg font-semibold">{detail.entity.name as string}</h2>
		<div class="mt-1 text-xs text-[var(--color-muted)]">{detail.file_id}</div>
	</header>

	<div class="flex flex-wrap gap-2 border-b border-[var(--color-border)] px-4 py-3 text-xs">
		<button class="rounded border border-[var(--color-border)] bg-white/5 px-2 py-1 hover:bg-white/10" onclick={onEditForm}>Edit form…</button>
		<button class="rounded border border-[var(--color-border)] bg-white/5 px-2 py-1 hover:bg-white/10" onclick={onEditBody}>Edit body…</button>
		<button class="ml-auto rounded border border-red-900 bg-red-900/30 px-2 py-1 text-[var(--color-bad)] hover:bg-red-900/50" onclick={onDelete}>Delete</button>
	</div>

	<div class="px-4 py-2">
		{#if tags.length > 0}
			<DetailField name="tags" value={tags} />
		{/if}
		{#each fields as [name, value]}
			<DetailField {name} {value} />
		{/each}
	</div>

	{#if detail.body.trim().length > 0}
		<div class="border-t border-[var(--color-border)] px-4 py-3">
			<BodyView body={detail.body} fileId={detail.file_id} />
		</div>
	{/if}

	{#if detail.backlinks.length > 0}
		<div class="border-t border-[var(--color-border)] px-4 py-3">
			<h3 class="mb-2 text-[10px] uppercase tracking-wider text-[var(--color-muted)]">Linked from</h3>
			<div class="flex flex-wrap gap-1.5">
				{#each detail.backlinks as bl}
					<button
						class="rounded border border-[var(--color-border)] bg-white/5 px-2 py-0.5 text-xs hover:bg-white/10"
						onclick={() => goto(backTarget(bl.src_file_id))}
					>
						{bl.src_file_id} <span class="text-[var(--color-muted)]">· {bl.field}</span>
					</button>
				{/each}
			</div>
		</div>
	{/if}
</div>
