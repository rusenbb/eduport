<script lang="ts">
	import { goto } from '$app/navigation';
	import { renderMarkdown } from '$lib/markdown';
	import { toggleCheckbox } from '$lib/api/checkbox';
	import { resolveEntity } from '$lib/api/entities';

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

	$effect(() => {
		if (!container) return;
		container.addEventListener('click', onClick);
		return () => container.removeEventListener('click', onClick);
	});

	async function onClick(event: MouseEvent) {
		const target = event.target as HTMLElement;
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

	async function onCheckbox(line: number, currentlyChecked: boolean) {
		const next = !currentlyChecked;
		try {
			await toggleCheckbox(fileId, line, next);
			// Optimistically rewrite the body locally so the UI reflects the change.
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
</script>

<div bind:this={container} class="prose prose-invert max-w-none text-sm" role="document">
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
