<script lang="ts">
	import { page } from '$app/state';
	import type { EntityType } from '$lib/types';

	const workspace = [
		{ href: '/', label: 'Dashboard', icon: '📊' },
		{ href: '/deadlines', label: 'Deadlines', icon: '🗓' }
	];

	const database: { href: string; label: string; icon: string; type: EntityType }[] = [
		{ href: '/program', label: 'Programs', icon: '🎓', type: 'program' },
		{ href: '/application', label: 'Applications', icon: '📨', type: 'application' },
		{ href: '/person', label: 'People', icon: '👤', type: 'person' },
		{ href: '/university', label: 'Universities', icon: '🏛', type: 'university' },
		{ href: '/lab', label: 'Labs', icon: '🔬', type: 'lab' },
		{ href: '/document', label: 'Documents', icon: '📄', type: 'document' },
		{ href: '/email', label: 'Emails', icon: '✉️', type: 'email' },
		{ href: '/notes', label: 'Notes', icon: '📝', type: 'note' }
	];

	function isActive(href: string): boolean {
		if (href === '/') return page.url.pathname === '/';
		return page.url.pathname.startsWith(href);
	}
</script>

<aside class="flex h-full w-[220px] flex-col gap-3 border-r border-[var(--color-border)] bg-[var(--color-panel)] p-3 text-sm">
	<div class="flex flex-col gap-1">
		<h4 class="px-2 pt-2 text-[10px] uppercase tracking-wider text-[var(--color-muted)]">Workspace</h4>
		{#each workspace as item}
			<a
				href={item.href}
				class="flex items-center gap-2 rounded px-2 py-1.5 text-[var(--color-text)] hover:bg-white/5"
				class:active={isActive(item.href)}
			>
				<span>{item.icon}</span>
				<span>{item.label}</span>
			</a>
		{/each}
	</div>

	<div class="flex flex-col gap-1">
		<h4 class="px-2 pt-2 text-[10px] uppercase tracking-wider text-[var(--color-muted)]">Database</h4>
		{#each database as item}
			<a
				href={item.href}
				class="flex items-center justify-between gap-2 rounded px-2 py-1.5 text-[var(--color-text)] hover:bg-white/5"
				class:active={isActive(item.href)}
			>
				<span class="flex items-center gap-2"><span>{item.icon}</span><span>{item.label}</span></span>
			</a>
		{/each}
	</div>

	<div class="mt-auto flex flex-col gap-1">
		<a href="/trash" class="flex items-center gap-2 rounded px-2 py-1.5 text-[var(--color-muted)] hover:bg-white/5" class:active={isActive('/trash')}>
			<span>🗑</span><span>Trash</span>
		</a>
	</div>
</aside>

<style>
	.active {
		background-color: rgba(108, 182, 255, 0.12);
		color: #ddebff;
	}
</style>
