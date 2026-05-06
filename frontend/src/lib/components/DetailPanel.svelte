<script lang="ts">
	import { goto } from '$app/navigation';
	import { getEntity, listEntities } from '$lib/api/entities';
	import { settings } from '$lib/stores/settings';
	import { cloneFileToFolder, openInObsidian, openPath, revealInFileManager, saveCopyAs } from '$lib/tauri';
	import type { EntityDetail } from '$lib/types';
	import DetailField from './DetailField.svelte';
	import BodyView from './BodyView.svelte';

	let {
		detail,
		focusMode = false,
		onEditForm,
		onEditBody,
		onFocus,
		onDelete
	}: {
		detail: EntityDetail;
		focusMode?: boolean;
		onEditForm?: () => void;
		onEditBody?: () => void;
		onFocus?: () => void;
		onDelete?: () => void;
	} = $props();

	const isDocument = $derived(detail.type === 'document');
	const filePath = $derived(detail.entity.file as string | undefined);
	let relatedEmails: EntityDetail[] = $state([]);
	let localBody = $state('');
	let actionMenuOpen = $state(false);

	function entityPath(): string | null {
		if (detail.path) return detail.path;
		if (!$settings) return null;
		return `${$settings.data_folder.replace(/\/$/, '')}/${detail.file_id}.md`;
	}

	function entityFilename(): string {
		return `${detail.file_id}.md`;
	}

	function absoluteFilePath(): string | null {
		if (!filePath || !$settings) return null;
		const root = $settings.data_folder.replace(/\/$/, '');
		return filePath.startsWith('/') ? filePath : `${root}/${filePath}`;
	}

	async function revealAttachment() {
		const abs = absoluteFilePath();
		if (!abs) return;
		try {
			await revealInFileManager(abs);
		} catch (e) {
			alert(`Reveal failed: ${e instanceof Error ? e.message : String(e)}`);
		}
	}

	async function revealEntityFile() {
		const path = entityPath();
		if (!path) return;
		actionMenuOpen = false;
		try {
			await revealInFileManager(path);
		} catch (e) {
			alert(`Reveal failed: ${e instanceof Error ? e.message : String(e)}`);
		}
	}

	async function cloneEntityFile() {
		const path = entityPath();
		if (!path) return;
		actionMenuOpen = false;
		try {
			await cloneFileToFolder(path, entityFilename());
		} catch (e) {
			alert(`Clone failed: ${e instanceof Error ? e.message : String(e)}`);
		}
	}

	function deleteEntityFile() {
		actionMenuOpen = false;
		onDelete?.();
	}

	async function openAttachment() {
		const abs = absoluteFilePath();
		if (!abs) return;
		try {
			await openPath(abs);
		} catch (e) {
			alert(`Open failed: ${e instanceof Error ? e.message : String(e)}`);
		}
	}

	async function saveAttachmentCopy() {
		const abs = absoluteFilePath();
		if (!abs) return;
		try {
			await saveCopyAs(abs, filePath?.split('/').pop() ?? 'attachment');
		} catch (e) {
			alert(`Save copy failed: ${e instanceof Error ? e.message : String(e)}`);
		}
	}

	async function openObsidian() {
		const vault = $settings?.obsidian_vault;
		if (!vault) return;
		await openInObsidian(vault, `${detail.file_id}.md`);
	}

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

	function backTarget(type: string | undefined, srcFileId: string): string {
		return `/${type ?? 'note'}/${encodeURIComponent(srcFileId)}`;
	}

	function relatedLinkForCurrentDetail(): string {
		return `[[${detail.file_id}]]`;
	}

	function emailReferencesCurrent(email: EntityDetail): boolean {
		const current = relatedLinkForCurrentDetail();
		const entity = email.entity;
		return (
			entity.related_application === current ||
			entity.related_program === current ||
			(Array.isArray(entity.related_people) && entity.related_people.includes(current)) ||
			(Array.isArray(entity.attachments) && entity.attachments.includes(current))
		);
	}

	async function loadRelatedEmails() {
		if (!['application', 'person', 'program', 'document'].includes(detail.type)) {
			relatedEmails = [];
			return;
		}
		try {
			const emails = await listEntities('email');
			const details = await Promise.all(emails.map((item) => getEntity('email', item.file_id).catch(() => null)));
			relatedEmails = details.filter((email): email is EntityDetail => !!email && emailReferencesCurrent(email));
		} catch {
			relatedEmails = [];
		}
	}

	$effect(() => {
		detail.file_id;
		localBody = detail.body;
		void loadRelatedEmails();
	});
</script>

<div class="flex h-full w-full flex-col overflow-auto bg-[var(--color-panel)]">
	<header class="flex items-start gap-3 border-b border-[var(--color-border)] p-4">
		<div class="min-w-0 flex-1">
			<h2 class="truncate text-lg font-semibold">{detail.entity.name as string}</h2>
			<div class="mt-1 truncate text-xs text-[var(--color-muted)]">{detail.file_id}</div>
		</div>
		{#if onFocus}
			<button class="rounded border border-[var(--color-border)] bg-white/5 px-2 py-1 text-xs hover:bg-white/10" onclick={onFocus}>
				{focusMode ? 'Exit focus' : 'Focus'}
			</button>
		{/if}
		<div class="relative">
			<button
				class="rounded border border-[var(--color-border)] bg-white/5 px-2 py-1 text-xs hover:bg-white/10"
				aria-label="More actions"
				onclick={() => (actionMenuOpen = !actionMenuOpen)}
			>
				...
			</button>
			{#if actionMenuOpen}
				<div class="absolute right-0 z-30 mt-1 w-44 overflow-hidden rounded border border-[var(--color-border)] bg-[var(--color-panel)] text-xs shadow-xl">
					<button class="block w-full px-3 py-2 text-left hover:bg-white/5" onclick={revealEntityFile}>
						Show in file manager
					</button>
					<button class="block w-full px-3 py-2 text-left hover:bg-white/5" onclick={cloneEntityFile}>
						Clone to folder
					</button>
					<button class="block w-full px-3 py-2 text-left text-[var(--color-bad)] hover:bg-red-900/30" onclick={deleteEntityFile}>
						Delete
					</button>
				</div>
			{/if}
		</div>
	</header>

	<div class="flex flex-wrap gap-2 border-b border-[var(--color-border)] px-4 py-3 text-xs">
		<button class="rounded border border-[var(--color-border)] bg-white/5 px-2 py-1 hover:bg-white/10" onclick={onEditForm}>Edit fields</button>
		<button class="rounded border border-[var(--color-border)] bg-white/5 px-2 py-1 hover:bg-white/10" onclick={onEditBody}>Edit body</button>
		{#if isDocument && filePath}
			<button
				class="rounded border border-[var(--color-border)] bg-white/5 px-2 py-1 hover:bg-white/10"
				onclick={openAttachment}
			>
				Open
			</button>
			<button
				class="rounded border border-[var(--color-border)] bg-white/5 px-2 py-1 hover:bg-white/10"
				onclick={revealAttachment}
			>
				Reveal in file manager
			</button>
			<button
				class="rounded border border-[var(--color-border)] bg-white/5 px-2 py-1 hover:bg-white/10"
				onclick={saveAttachmentCopy}
			>
				Save copy as
			</button>
		{/if}
		{#if $settings?.obsidian_vault}
			<button
				class="rounded border border-[var(--color-border)] bg-white/5 px-2 py-1 hover:bg-white/10"
				onclick={openObsidian}
			>
				Open in Obsidian
			</button>
		{/if}
	</div>

	<div class="px-4 py-2">
		{#if tags.length > 0}
			<DetailField name="tags" value={tags} />
		{/if}
		{#each fields as [name, value]}
			<DetailField {name} {value} />
		{/each}
	</div>

	{#if localBody.trim().length > 0}
		<div class="border-t border-[var(--color-border)] px-4 py-3">
			<BodyView body={localBody} fileId={detail.file_id} onChange={(newBody) => (localBody = newBody)} />
		</div>
	{/if}

	{#if relatedEmails.length > 0}
		<div class="border-t border-[var(--color-border)] px-4 py-3">
			<h3 class="mb-2 text-[10px] uppercase tracking-wider text-[var(--color-muted)]">Communications</h3>
			<div class="grid gap-1.5">
				{#each relatedEmails as email}
					<button
						class="rounded border border-[var(--color-border)] bg-white/5 px-2 py-1 text-left text-xs hover:bg-white/10"
						onclick={() => goto(`/email/${email.file_id}`)}
					>
						<div class="font-medium">{email.entity.subject as string}</div>
						<div class="text-[10px] text-[var(--color-muted)]">{email.entity.date as string} · {email.entity.direction as string}</div>
					</button>
				{/each}
			</div>
		</div>
	{/if}

	{#if detail.backlinks.length > 0}
		<div class="border-t border-[var(--color-border)] px-4 py-3">
			<h3 class="mb-2 text-[10px] uppercase tracking-wider text-[var(--color-muted)]">Linked from</h3>
			<div class="flex flex-wrap gap-1.5">
				{#each detail.backlinks as bl}
					<button
						class="rounded border border-[var(--color-border)] bg-white/5 px-2 py-0.5 text-xs hover:bg-white/10"
						onclick={() => goto(backTarget(bl.type, bl.src_file_id))}
					>
						{bl.name ?? bl.src_file_id} <span class="text-[var(--color-muted)]">· {bl.field}</span>
					</button>
				{/each}
			</div>
		</div>
	{/if}
</div>
