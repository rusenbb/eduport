<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolveEntity } from '$lib/api/entities';
	import { COLOR_CLASSES, type Property, type SelectOption } from '$lib/types/schema';
	import { onMount } from 'svelte';
	import Icon from '../Icon.svelte';

	let {
		prop,
		value
	}: {
		prop: Property;
		value: unknown;
	} = $props();

	function optionFor(prop: Property, v: string): SelectOption | null {
		if (prop.type !== 'single-select' && prop.type !== 'multi-select') return null;
		return prop.options.find((o) => o.value === v) ?? null;
	}

	function targetOf(link: string): string | null {
		const m = /^\[\[(.+)\]\]$/.exec(link.trim());
		return m ? m[1] : null;
	}

	let resolvedRelation: { type: string; name: string } | null = $state(null);
	$effect(() => {
		resolvedRelation = null;
		if (prop.type !== 'relation' || typeof value !== 'string') return;
		const target = targetOf(value);
		if (!target) return;
		void resolveEntity(target).then(
			(r) => (resolvedRelation = { type: r.type, name: r.name }),
			() => (resolvedRelation = null)
		);
	});

	// Multi-relation: when the value is a list of wikilinks (built-in
	// plural-relation fields like `labs`, `documents`, `people`,
	// `attachments`, `related_people`), resolve each one in parallel.
	let resolvedList: Array<{ type: string; name: string } | null> = $state([]);
	$effect(() => {
		resolvedList = [];
		if (prop.type !== 'relation' || !Array.isArray(value)) return;
		const links = value.filter((v): v is string => typeof v === 'string');
		const buf: Array<{ type: string; name: string } | null> = links.map(() => null);
		links.forEach((link, i) => {
			const target = targetOf(link);
			if (!target) return;
			void resolveEntity(target).then(
				(r) => {
					buf[i] = { type: r.type, name: r.name };
					resolvedList = [...buf];
				},
				() => {}
			);
		});
		resolvedList = buf;
	});

	function navigate(link: string, resolved: { type: string; name: string } | null) {
		const target = targetOf(link);
		if (!target || !resolved) return;
		void goto(`/${resolved.type}/${target}`);
	}
</script>

{#if value === undefined || value === null || value === ''}
	<span class="text-xs italic text-[var(--color-muted)]">empty</span>
{:else if prop.type === 'text' || prop.type === 'number'}
	<span class="text-sm">{String(value)}</span>
	{#if prop.type === 'number' && prop.unit}
		<span class="ml-1 text-xs text-[var(--color-muted)]">{prop.unit}</span>
	{/if}
{:else if prop.type === 'date'}
	<span class="text-sm">{String(value)}</span>
{:else if prop.type === 'checkbox'}
	<span class="inline-flex items-center gap-1 text-sm">
		<input type="checkbox" checked={value === true} disabled />
		<span>{value === true ? 'Yes' : 'No'}</span>
	</span>
{:else if prop.type === 'url'}
	<a class="break-all text-sm text-[var(--color-accent)] underline hover:opacity-80" href={String(value)} target="_blank" rel="noreferrer">
		{String(value)}
	</a>
{:else if prop.type === 'single-select'}
	{@const opt = optionFor(prop, String(value))}
	{#if opt}
		{@const c = COLOR_CLASSES[opt.color]}
		<span class="inline-flex items-center rounded border px-2 py-0.5 text-xs {c.bg} {c.text} {c.border}">
			{opt.label}
		</span>
	{:else}
		<span class="inline-flex items-center rounded border border-[var(--color-border)] bg-white/5 px-2 py-0.5 text-xs">
			{String(value)}
		</span>
	{/if}
{:else if prop.type === 'multi-select' && Array.isArray(value)}
	<div class="flex flex-wrap gap-1.5">
		{#each value as v}
			{@const opt = optionFor(prop, String(v))}
			{#if opt}
				{@const c = COLOR_CLASSES[opt.color]}
				<span class="inline-flex items-center rounded border px-2 py-0.5 text-xs {c.bg} {c.text} {c.border}">
					{opt.label}
				</span>
			{:else}
				<span class="inline-flex items-center rounded border border-[var(--color-border)] bg-white/5 px-2 py-0.5 text-xs">
					{String(v)}
				</span>
			{/if}
		{/each}
	</div>
{:else if prop.type === 'relation' && typeof value === 'string'}
	<button
		type="button"
		class="inline-flex items-center gap-1 rounded border border-[var(--color-border)] bg-white/5 px-2 py-0.5 text-xs hover:bg-white/10"
		onclick={() => navigate(value, resolvedRelation)}
		disabled={!resolvedRelation}
	>
		<Icon name="link" class="text-[var(--color-muted)]" />
		<span>{resolvedRelation?.name ?? targetOf(value) ?? value}</span>
		{#if resolvedRelation}
			<span class="text-[10px] text-[var(--color-muted)]">· {resolvedRelation.type}</span>
		{/if}
	</button>
{:else if prop.type === 'relation' && Array.isArray(value)}
	<div class="flex flex-wrap gap-1.5">
		{#each value as link, i}
			{#if typeof link === 'string'}
				{@const r = resolvedList[i]}
				<button
					type="button"
					class="inline-flex items-center gap-1 rounded border border-[var(--color-border)] bg-white/5 px-2 py-0.5 text-xs hover:bg-white/10"
					onclick={() => navigate(link, r)}
					disabled={!r}
				>
					<Icon name="link" class="text-[var(--color-muted)]" />
					<span>{r?.name ?? targetOf(link) ?? link}</span>
					{#if r}
						<span class="text-[10px] text-[var(--color-muted)]">· {r.type}</span>
					{/if}
				</button>
			{/if}
		{/each}
	</div>
{:else}
	<span class="text-xs text-[var(--color-muted)]">{JSON.stringify(value)}</span>
{/if}
