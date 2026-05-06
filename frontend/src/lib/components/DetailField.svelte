<script lang="ts">
	import { goto } from '$app/navigation';
	import { inferTypeFromField, targetOf } from '$lib/entities/meta';
	import { filters } from '$lib/stores/filters';

	let { name, value }: { name: string; value: unknown } = $props();

	function isWikilink(v: unknown): v is string {
		return typeof v === 'string' && /^\[\[[^\]\[]+\]\]$/.test(v);
	}

	function navigate(link: string) {
		const target = targetOf(link);
		if (!target) return;
		const t = inferTypeFromField(name);
		goto(`/${t}/${encodeURIComponent(target)}`);
	}

	async function copy(text: string) {
		await navigator.clipboard?.writeText(text).catch(() => {});
	}
</script>

<div class="border-b border-[var(--color-border)] py-3">
	<div class="text-[10px] uppercase tracking-wider text-[var(--color-muted)]">{name}</div>
	<div class="mt-1 text-sm">
		{#if isWikilink(value)}
			<button class="text-[var(--color-accent)] hover:underline" onclick={() => navigate(value as string)}>
				{targetOf(value)}
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
								{targetOf(v)}
							</button>
						{:else if typeof v === 'object' && v !== null}
							<div class="flex items-center gap-1 rounded border border-[var(--color-border)] bg-white/5 px-2 py-0.5 text-xs">
								{#if 'url' in v && typeof v.url === 'string'}
									<a href={v.url} target="_blank" rel="noopener" class="text-[var(--color-accent)] hover:underline">
										{'label' in v ? v.label : v.url}
									</a>
								{:else if 'email' in v && typeof v.email === 'string'}
									<a href={`mailto:${v.email}`} class="text-[var(--color-accent)] hover:underline">
										{'label' in v ? v.label : v.email}
									</a>
									<button class="text-[10px] text-[var(--color-muted)] hover:text-[var(--color-text)]" onclick={() => copy(v.email)}>
										Copy
									</button>
								{:else}
									<span>{JSON.stringify(v)}</span>
								{/if}
							</div>
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
