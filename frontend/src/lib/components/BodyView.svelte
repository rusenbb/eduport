<script lang="ts">
	import { goto } from '$app/navigation';
	import { renderMarkdown } from '$lib/markdown';
	import { toggleCheckbox } from '$lib/api/checkbox';
	import { getEntity, resolveEntity } from '$lib/api/entities';
	import { extractIcon } from '$lib/entities/cosmetics';
	import type { EntityDetail } from '$lib/types';

	let {
		body,
		fileId,
		onChange
	}: {
		body: string;
		fileId: string;
		onChange?: (newBody: string) => void;
	} = $props();

	const rendered = $derived(renderMarkdown(body));
	let container: HTMLDivElement;

	// Lightweight in-component cache so re-hovering a link doesn't re-fetch.
	// Cleared by component teardown; we're not trying to share across panels.
	const previewCache = new Map<string, EntityDetail | null>();
	let hoverTimer: ReturnType<typeof setTimeout> | null = null;
	let preview: { target: string; detail: EntityDetail | null; left: number; top: number } | null =
		$state(null);

	$effect(() => {
		if (!container) return;
		container.addEventListener('click', onClick);
		container.addEventListener('mouseover', onHover);
		container.addEventListener('mouseout', onLeave);
		return () => {
			container.removeEventListener('click', onClick);
			container.removeEventListener('mouseover', onHover);
			container.removeEventListener('mouseout', onLeave);
			if (hoverTimer) clearTimeout(hoverTimer);
		};
	});

	async function onClick(event: MouseEvent) {
		const target = event.target as HTMLElement;
		const embed = target.closest('.view-embed') as HTMLElement | null;
		if (embed) {
			event.preventDefault();
			const name = embed.getAttribute('data-view');
			if (name) {
				// Embed-as-card MVP: navigate to a generic landing path
				// where the saved view can be picked. The full inline
				// rendering of the view's filtered entity list lives in
				// a follow-up — this step ships the round-trip + the
				// affordance, not the inline list.
				goto(`/?embed=${encodeURIComponent(name)}`);
			}
			return;
		}
		if (target.tagName === 'A' && target.classList.contains('wikilink')) {
			event.preventDefault();
			const t = target.getAttribute('data-target');
			if (t) {
				try {
					const resolved = await resolveEntity(t);
					goto(`/${resolved.type}/${encodeURIComponent(resolved.file_id)}`);
				} catch {
					goto(`/note/${encodeURIComponent(t)}`);
				}
			}
		}
	}

	function onHover(event: MouseEvent) {
		const target = event.target as HTMLElement;
		if (target.tagName !== 'A' || !target.classList.contains('wikilink')) return;
		const t = target.getAttribute('data-target');
		if (!t) return;
		if (hoverTimer) clearTimeout(hoverTimer);
		hoverTimer = setTimeout(() => void showPreview(target, t), 250);
	}

	function onLeave(event: MouseEvent) {
		const target = event.target as HTMLElement;
		if (target.tagName !== 'A' || !target.classList.contains('wikilink')) return;
		if (hoverTimer) {
			clearTimeout(hoverTimer);
			hoverTimer = null;
		}
		preview = null;
	}

	async function showPreview(linkEl: HTMLElement, t: string) {
		const rect = linkEl.getBoundingClientRect();
		const left = rect.left;
		const top = rect.bottom + 4;
		let detail: EntityDetail | null;
		if (previewCache.has(t)) {
			detail = previewCache.get(t) ?? null;
		} else {
			try {
				const resolved = await resolveEntity(t);
				detail = await getEntity(resolved.type, resolved.file_id);
			} catch {
				detail = null;
			}
			previewCache.set(t, detail);
		}
		// Only show if the user is still hovering this link (preview === null
		// means they already moved out).
		if (hoverTimer !== null) preview = { target: t, detail, left, top };
	}

	async function onCheckbox(line: number, currentlyChecked: boolean) {
		const next = !currentlyChecked;
		try {
			await toggleCheckbox(fileId, line, next);
			const lines = body.split('\n');
			const old = lines[line];
			if (old) {
				lines[line] = next
					? old.replace(/^- \[ \]/, '- [x]')
					: old.replace(/^- \[[xX]\]/, '- [ ]');
				onChange?.(lines.join('\n'));
			}
		} catch (e) {
			console.error('checkbox toggle failed', e);
		}
	}

	function previewSnippet(detail: EntityDetail | null): string {
		if (!detail) return 'Note: not yet created';
		const body = (detail.entity as { body?: string }).body ?? '';
		const firstLine = body.split('\n').find((l) => l.trim().length > 0) ?? '';
		return firstLine.slice(0, 140);
	}
</script>

<div bind:this={container} class="body-prose text-sm" role="document">
	{#if rendered.checkboxes.length > 0}
		<ul class="not-prose mb-4 list-none space-y-1 pl-0">
			{#each rendered.checkboxes as cb}
				<li class="flex items-start gap-2">
					<input
						type="checkbox"
						class="mt-1 accent-[var(--color-accent)]"
						checked={cb.checked}
						onchange={() => onCheckbox(cb.line, cb.checked)}
					/>
					<span class:line-through={cb.checked} class:opacity-60={cb.checked}>{cb.text}</span>
				</li>
			{/each}
		</ul>
	{/if}
	{@html rendered.html}
</div>

{#if preview}
	<div
		class="pointer-events-none fixed z-40 w-72 rounded border border-[var(--color-border)] bg-[var(--color-panel)] p-3 text-xs shadow-lg"
		style="left: {preview.left}px; top: {preview.top}px"
		role="tooltip"
	>
		<div class="flex items-center gap-2">
			{#if extractIcon(preview.detail)}
				<span class="text-base leading-none" aria-hidden="true">{extractIcon(preview.detail)}</span>
			{/if}
			<div class="min-w-0 flex-1">
				<div class="truncate font-medium">{preview.detail?.entity.name ?? preview.target}</div>
				{#if preview.detail}
					<div class="text-[10px] uppercase tracking-wider text-[var(--color-muted)]">
						{preview.detail.type}
					</div>
				{/if}
			</div>
		</div>
		{#if previewSnippet(preview.detail)}
			<p class="mt-2 line-clamp-3 text-[var(--color-muted)]">{previewSnippet(preview.detail)}</p>
		{/if}
	</div>
{/if}
