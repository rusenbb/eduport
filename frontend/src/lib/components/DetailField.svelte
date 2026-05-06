<script lang="ts">
	import { goto } from '$app/navigation';
	import { filters } from '$lib/stores/filters';

	let { name, value }: { name: string; value: unknown } = $props();

	function isWikilink(v: unknown): v is string {
		return typeof v === 'string' && /^\[\[[^\]\[]+\]\]$/.test(v);
	}

	function targetOf(link: string): string {
		return link.slice(2, -2).trim();
	}

	function inferTypeFromTarget(target: string): string {
		// Crude: look at the field name to infer the entity type for routing.
		// Falls back to 'university' which always renders.
		const map: Record<string, string> = {
			university: 'university',
			labs: 'lab',
			people: 'person',
			program: 'program',
			recommender: 'person',
			related_program: 'program',
			related_application: 'application',
			related_people: 'person',
			documents: 'document',
			attachments: 'document',
			in_reply_to: 'email'
		};
		return map[name] ?? 'university';
	}

	function navigate(link: string) {
		const t = inferTypeFromTarget(targetOf(link));
		goto(`/${t}/${encodeURIComponent(targetOf(link))}`);
	}
</script>

<div class="border-b border-[var(--color-border)] py-3">
	<div class="text-[10px] uppercase tracking-wider text-[var(--color-muted)]">{name}</div>
	<div class="mt-1 text-sm">
		{#if isWikilink(value)}
			<button class="text-[var(--color-accent)] hover:underline" onclick={() => navigate(value as string)}>
				{targetOf(value as string)}
			</button>
		{:else if Array.isArray(value)}
			{#if value.length === 0}
				<span class="text-[var(--color-muted)]">—</span>
			{:else}
				<div class="flex flex-wrap gap-1.5">
					{#each value as v}
						{#if isWikilink(v)}
							<button
								class="rounded border border-[var(--color-accent)]/40 bg-[var(--color-accent)]/10 px-2 py-0.5 text-xs text-[var(--color-accent)] hover:bg-[var(--color-accent)]/20"
								onclick={() => navigate(v as string)}
							>
								{targetOf(v as string)}
							</button>
						{:else if typeof v === 'object' && v !== null}
							<span class="rounded border border-[var(--color-border)] bg-white/5 px-2 py-0.5 text-xs">
								{JSON.stringify(v)}
							</span>
						{:else if name === 'tags' && typeof v === 'string'}
							<button
								class="rounded-full border border-[var(--color-border)] bg-white/5 px-2 py-0.5 text-xs hover:bg-white/10"
								onclick={() => filters.addTag(v as string)}
							>
								{v}
							</button>
						{:else}
							<span class="rounded border border-[var(--color-border)] bg-white/5 px-2 py-0.5 text-xs">
								{String(v)}
							</span>
						{/if}
					{/each}
				</div>
			{/if}
		{:else if typeof value === 'string' && /^https?:\/\//.test(value)}
			<a href={value} target="_blank" rel="noopener" class="text-[var(--color-accent)] hover:underline">{value}</a>
		{:else if value === null || value === undefined}
			<span class="text-[var(--color-muted)]">—</span>
		{:else if typeof value === 'object'}
			<pre class="overflow-auto text-xs text-[var(--color-muted)]">{JSON.stringify(value, null, 2)}</pre>
		{:else}
			<span>{String(value)}</span>
		{/if}
	</div>
</div>
