<script lang="ts">
	import { goto } from '$app/navigation';
	import {
		entityChildren,
		getEntity,
		listEntities,
		resolveEntity,
		updateEntity
	} from '$lib/api/entities';
	import { schemaStore } from '$lib/stores/schema';
	import { settings } from '$lib/stores/settings';
	import { toasts } from '$lib/stores/toasts';
	import { cloneFileToFolder, openInObsidian, openPath, revealInFileManager, saveCopyAs } from '$lib/tauri';
	import type { EntityDetail, EntityListItem, EntityType } from '$lib/types';
	import { onMount } from 'svelte';
	import { extractIcon, extractCover } from '$lib/entities/cosmetics';
	import DetailField from './DetailField.svelte';
	import BodyView from './BodyView.svelte';
	import ResourceListEditor from './ResourceListEditor.svelte';
	import TagPicker from './TagPicker.svelte';
	import PropertyConfigDialog from './properties/PropertyConfigDialog.svelte';
	import PropertyEditor from './properties/PropertyEditor.svelte';
	import PropertyTypeIcon from './properties/PropertyTypeIcon.svelte';
	import PropertyValue from './properties/PropertyValue.svelte';
	import PropertyWarningChip from './properties/PropertyWarningChip.svelte';

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
	const icon = $derived(extractIcon(detail));
	const cover = $derived(extractCover(detail));

	// Hierarchy: breadcrumb walks up via `entity.parent` (resolved by
	// file_id, not name — anchors are stable across renames). Sub-pages
	// are entities whose `parent` field equals our file_id.
	type Crumb = { file_id: string; type: EntityType; name: string };
	let ancestors: Crumb[] = $state([]);
	let ancestorCycle = $state(false);
	let children: EntityListItem[] = $state([]);

	async function loadHierarchy() {
		ancestors = [];
		ancestorCycle = false;
		children = [];

		// Walk up. Limit to 16 hops so a runaway never burns the UI.
		const seen = new Set<string>([detail.file_id]);
		const chain: Crumb[] = [];
		const initial = (detail.entity as { parent?: unknown }).parent;
		let cursor: string | undefined = typeof initial === 'string' ? initial : undefined;
		let depth = 0;
		while (cursor && depth < 16) {
			const here: string = cursor;
			if (seen.has(here)) {
				ancestorCycle = true;
				break;
			}
			seen.add(here);
			depth++;
			try {
				const resolved = await resolveEntity(here);
				chain.unshift({
					file_id: resolved.file_id,
					type: resolved.type,
					name: resolved.name
				});
				const fetched = await getEntity(resolved.type, resolved.file_id);
				const next = (fetched.entity as { parent?: unknown }).parent;
				cursor = typeof next === 'string' ? next : undefined;
			} catch {
				// Unresolved parent — surface as a dangling crumb.
				chain.unshift({ file_id: here, type: 'note', name: here });
				break;
			}
		}
		ancestors = chain;

		try {
			children = await entityChildren(detail.file_id);
		} catch {
			children = [];
		}
	}

	$effect(() => {
		// Re-run whenever the open entity changes.
		const _ = detail.file_id;
		void loadHierarchy();
	});
	let relatedEmails: EntityDetail[] = $state([]);
	let threadInReplyTo: EntityDetail | null = $state(null);
	let threadReplies: EntityDetail[] = $state([]);
	let localBody = $state('');
	let actionMenuOpen = $state(false);
	// The action menu used to be `position: absolute` inside the
	// detail panel header, which sits under three nested overflow
	// contexts (workspace grid → aside → panel root with overflow-auto).
	// That clipped the dropdown so items at the bottom were unreachable
	// and the menu visually rendered behind the list column header.
	// Switch to `position: fixed` with coordinates computed from the
	// button's bounding rect so it escapes every clipping ancestor.
	let menuButton: HTMLButtonElement | null = $state(null);
	let menuPos: { top: number; right: number } = $state({ top: 0, right: 0 });

	function openActionMenu() {
		if (actionMenuOpen) {
			actionMenuOpen = false;
			return;
		}
		if (menuButton) {
			const r = menuButton.getBoundingClientRect();
			menuPos = { top: r.bottom + 4, right: window.innerWidth - r.right };
		}
		actionMenuOpen = true;
	}

	// Close the menu on outside clicks, Escape, scroll, or resize so
	// it can't end up stranded after the page state shifts under it.
	$effect(() => {
		if (!actionMenuOpen) return;
		function onDocClick(e: MouseEvent) {
			const target = e.target as Node;
			if (menuButton?.contains(target)) return;
			// Defer so the same click that opened it doesn't immediately close it.
			actionMenuOpen = false;
		}
		function onKey(e: KeyboardEvent) {
			if (e.key === 'Escape') actionMenuOpen = false;
		}
		function onScrollOrResize() {
			actionMenuOpen = false;
		}
		// Wait one tick so the click that opened doesn't fire on this listener.
		const id = window.setTimeout(() => {
			window.addEventListener('click', onDocClick);
		}, 0);
		window.addEventListener('keydown', onKey);
		window.addEventListener('resize', onScrollOrResize);
		window.addEventListener('scroll', onScrollOrResize, true);
		return () => {
			window.clearTimeout(id);
			window.removeEventListener('click', onDocClick);
			window.removeEventListener('keydown', onKey);
			window.removeEventListener('resize', onScrollOrResize);
			window.removeEventListener('scroll', onScrollOrResize, true);
		};
	});

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
			toasts.error('Reveal failed', e instanceof Error ? e.message : String(e));
		}
	}

	async function revealEntityFile() {
		const path = entityPath();
		if (!path) return;
		actionMenuOpen = false;
		try {
			await revealInFileManager(path);
		} catch (e) {
			toasts.error('Reveal failed', e instanceof Error ? e.message : String(e));
		}
	}

	async function cloneEntityFile() {
		const path = entityPath();
		if (!path) return;
		actionMenuOpen = false;
		try {
			await cloneFileToFolder(path, entityFilename());
		} catch (e) {
			toasts.error('Clone failed', e instanceof Error ? e.message : String(e));
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
			toasts.error('Open failed', e instanceof Error ? e.message : String(e));
		}
	}

	async function saveAttachmentCopy() {
		const abs = absoluteFilePath();
		if (!abs) return;
		try {
			await saveCopyAs(abs, filePath?.split('/').pop() ?? 'attachment');
		} catch (e) {
			toasts.error('Save copy failed', e instanceof Error ? e.message : String(e));
		}
	}

	async function openObsidian() {
		const vault = $settings?.obsidian_vault;
		if (!vault) return;
		await openInObsidian(vault, `${detail.file_id}.md`);
	}

	// Build the list of custom-property keys for this entity type from the schema
	// and split the entity's frontmatter into "built-in" fields (rendered by the
	// existing DetailField component) vs "custom" fields (rendered with
	// PropertyValue / PropertyEditor below).
	const customPropertiesForType = $derived(
		$schemaStore.schema?.types[detail.type]?.properties ?? []
	);
	const customKeys = $derived(new Set(customPropertiesForType.map((p) => p.key)));
	const builtinKeys = $derived(
		new Set($schemaStore.schema?.types[detail.type]?.builtin_keys ?? [])
	);

	// vaultdb's typed read path round-trips the entity through
	// `record_to_json`, which injects the virtual fields `_name`,
	// `_path`, `_folder`, `_modified`, `_created`. They land in the
	// flattened `custom` map and serialize back out at the top level.
	// They're useful for debugging / power users but visual noise by
	// default, so we split them into their own collapsible "System"
	// section.
	function isVirtualKey(k: string): boolean {
		return k.startsWith('_');
	}
	const fields = $derived(
		Object.entries(detail.entity).filter(
			([k]) =>
				k !== 'name' && k !== 'tags' && !customKeys.has(k) && !isVirtualKey(k)
		)
	);
	const systemFields = $derived(
		Object.entries(detail.entity).filter(([k]) => isVirtualKey(k))
	);
	let showSystemFields = $state(false);

	// Inline editing — one property at a time.
	let editingKey: string | null = $state(null);
	let editingValue: unknown = $state(undefined);

	function startEdit(key: string, currentValue: unknown) {
		editingKey = key;
		editingValue = currentValue ?? null;
	}

	function cancelEdit() {
		editingKey = null;
		editingValue = undefined;
	}

	async function commitEdit() {
		if (editingKey === null) return;
		const key = editingKey;
		const next = editingValue;
		const newFm: Record<string, unknown> = { ...(detail.entity as Record<string, unknown>) };
		// Drop empty values rather than writing nulls — keeps YAML clean.
		if (next === null || next === undefined || next === '' ||
			(Array.isArray(next) && next.length === 0)) {
			delete newFm[key];
		} else {
			newFm[key] = next;
		}
		try {
			await updateEntity(detail.type, detail.file_id, newFm, detail.body);
			editingKey = null;
			editingValue = undefined;
			// Refetch to pick up new value_warnings.
			const fresh = await getEntity(detail.type, detail.file_id);
			detail.entity = fresh.entity;
			detail.value_warnings = fresh.value_warnings;
		} catch (e) {
			toasts.error('Save failed', e instanceof Error ? e.message : String(e));
		}
	}

	onMount(() => {
		// Make sure the schema is loaded so we can render custom properties.
		void schemaStore.load();
	});

	function warningsForKey(key: string) {
		return (detail.value_warnings ?? []).filter((w) => w.key === key);
	}

	let addingProperty = $state(false);

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

	async function loadEmailThread() {
		if (detail.type !== 'email') {
			threadInReplyTo = null;
			threadReplies = [];
			return;
		}
		const selfLink = relatedLinkForCurrentDetail();
		const inReplyToLink = (detail.entity.in_reply_to as string | null) ?? null;
		try {
			const emails = await listEntities('email');
			const details = await Promise.all(emails.map((item) => getEntity('email', item.file_id).catch(() => null)));
			const valid = details.filter((email): email is EntityDetail => !!email);
			// Backward: the email this one replies to (one hop).
			threadInReplyTo = inReplyToLink
				? (valid.find((email) => relatedLinkFor(email) === inReplyToLink) ?? null)
				: null;
			// Forward: emails whose in_reply_to points at us (one hop each).
			threadReplies = valid.filter((email) => email.entity.in_reply_to === selfLink);
		} catch {
			threadInReplyTo = null;
			threadReplies = [];
		}
	}

	function relatedLinkFor(email: EntityDetail): string {
		return `[[${email.file_id}]]`;
	}

	$effect(() => {
		detail.file_id;
		localBody = detail.body;
		void loadRelatedEmails();
		void loadEmailThread();
	});
</script>

<div class="flex h-full w-full flex-col overflow-auto bg-[var(--color-panel)]">
	{#if cover}
		<div
			class="h-32 w-full border-b border-[var(--color-border)] bg-cover bg-center"
			style="background-image: url('{cover}')"
			aria-hidden="true"
		></div>
	{/if}
	{#if ancestors.length > 0 || ancestorCycle}
		<nav
			class="flex flex-wrap items-center gap-1 border-b border-[var(--color-border)] px-4 py-2 text-xs text-[var(--color-muted)]"
			aria-label="Page hierarchy"
		>
			{#each ancestors as crumb (crumb.file_id)}
				<a
					class="rounded px-1 hover:bg-white/5 hover:text-[var(--color-text)]"
					href="/{crumb.type}/{crumb.file_id}"
				>{crumb.name}</a>
				<span aria-hidden="true">/</span>
			{/each}
			<span class="text-[var(--color-text)]">{detail.entity.name as string}</span>
			{#if ancestorCycle}
				<span class="ml-2 rounded bg-[var(--color-bad)]/20 px-1.5 py-0.5 text-[10px] uppercase tracking-wider text-[var(--color-bad)]">cycle</span>
			{/if}
		</nav>
	{/if}
	<header class="flex items-start gap-3 border-b border-[var(--color-border)] p-4">
		{#if icon}
			<span class="text-3xl leading-none" aria-hidden="true">{icon}</span>
		{/if}
		<div class="min-w-0 flex-1">
			<h2 class="truncate text-lg font-semibold">{detail.entity.name as string}</h2>
			<code class="mt-1 block truncate text-xs text-[var(--color-muted)]">{detail.file_id}</code>
		</div>
		{#if onFocus}
			<button class="rounded border border-[var(--color-border)] bg-white/5 px-2 py-1 text-xs hover:bg-white/10" onclick={onFocus}>
				{focusMode ? 'Exit focus' : 'Focus'}
			</button>
		{/if}
		<button
			bind:this={menuButton}
			class="rounded border border-[var(--color-border)] bg-white/5 px-2 py-1 text-xs hover:bg-white/10"
			aria-label="More actions"
			aria-haspopup="menu"
			aria-expanded={actionMenuOpen}
			onclick={openActionMenu}
		>
			...
		</button>
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
		<!-- Tags row: read view shows clickable chips; clicking the
		pencil flips into TagPicker via the shared editingKey/Value
		state used by custom properties below. Tag membership writes
		back through updateEntity → eduport-tauri → vaultdb. -->
		<div class="group relative border-b border-[var(--color-border)] py-3">
			<div class="flex items-center justify-between">
				<div class="text-[10px] uppercase tracking-wider text-[var(--color-muted)]">tags</div>
				{#if editingKey !== 'tags'}
					<button
						class="text-[10px] text-[var(--color-muted)] opacity-0 hover:text-[var(--color-text)] group-hover:opacity-100"
						onclick={() => startEdit('tags', tags)}
						aria-label="Edit tags"
					>
						<span class="text-[10px]">✎ Edit</span>
					</button>
				{/if}
			</div>
			<div class="mt-1">
				{#if editingKey === 'tags'}
					<TagPicker
						bind:tags={
							() => (Array.isArray(editingValue) ? (editingValue as string[]) : []),
							(next) => (editingValue = next)
						}
					/>
					<div class="mt-2 flex gap-1 text-[10px]">
						<button
							class="rounded border border-[var(--color-accent)] bg-[var(--color-accent)]/15 px-2 py-1 text-[var(--color-accent)] hover:bg-[var(--color-accent)]/25"
							onclick={() => {
								// Tags need special handling: keep the type-tag
								// (eduport-type/X) and any eduport-doctype/* on
								// save — they're invisible to the picker but
								// part of the wire shape.
								const reserved = (
									Array.isArray(detail.entity.tags) ? detail.entity.tags : []
								).filter(
									(t) =>
										typeof t === 'string' &&
										(t.startsWith('eduport-type/') ||
											t.startsWith('eduport-doctype/'))
								);
								editingValue = [
									...reserved,
									...(Array.isArray(editingValue) ? (editingValue as string[]) : [])
								];
								void commitEdit();
							}}>Save</button
						>
						<button
							class="rounded border border-[var(--color-border)] px-2 py-1 hover:bg-white/5"
							onclick={cancelEdit}>Cancel</button
						>
					</div>
				{:else}
					<DetailField name="tags" value={tags} />
				{/if}
			</div>
		</div>
		<!-- Non-schema built-ins (today: links, emails — anything
		whose shape doesn't fit the scalar Property variants). Edit
		flips into ResourceListEditor when the value is a list of
		{label, url} / {label, email} records; otherwise falls back to
		the existing DetailField read view (no inline edit yet for
		odd one-off shapes). -->
		{#each fields as [name, value]}
			{@const isResources = name === 'links' || name === 'emails'}
			<div class="group relative border-b border-[var(--color-border)] py-3">
				<div class="flex items-center justify-between">
					<div class="text-[10px] uppercase tracking-wider text-[var(--color-muted)]">{name}</div>
					{#if isResources && editingKey !== name}
						<button
							class="text-[10px] text-[var(--color-muted)] opacity-0 hover:text-[var(--color-text)] group-hover:opacity-100"
							onclick={() => startEdit(name, Array.isArray(value) ? value : [])}
							aria-label={`Edit ${name}`}
						>
							<span class="text-[10px]">✎ Edit</span>
						</button>
					{/if}
				</div>
				<div class="mt-1">
					{#if isResources && editingKey === name}
						<ResourceListEditor
							mode={name === 'emails' ? 'email' : 'link'}
							values={Array.isArray(editingValue)
								? (editingValue as Record<string, unknown>[])
								: []}
							onChange={(next) => (editingValue = next)}
						/>
						<div class="mt-2 flex gap-1 text-[10px]">
							<button
								class="rounded border border-[var(--color-accent)] bg-[var(--color-accent)]/15 px-2 py-1 text-[var(--color-accent)] hover:bg-[var(--color-accent)]/25"
								onclick={commitEdit}>Save</button
							>
							<button
								class="rounded border border-[var(--color-border)] px-2 py-1 hover:bg-white/5"
								onclick={cancelEdit}>Cancel</button
							>
						</div>
					{:else}
						<div class="-mt-1">
							<DetailField {name} {value} />
						</div>
					{/if}
				</div>
			</div>
		{/each}
	</div>

	{#if systemFields.length > 0}
		<div class="border-t border-[var(--color-border)] px-4 py-2">
			<button
				type="button"
				class="text-[10px] uppercase tracking-wider text-[var(--color-muted)] hover:text-[var(--color-text)]"
				onclick={() => (showSystemFields = !showSystemFields)}
			>
				{showSystemFields ? '▾' : '▸'} System ({systemFields.length})
			</button>
			{#if showSystemFields}
				<div class="mt-2">
					{#each systemFields as [name, value]}
						<DetailField {name} {value} />
					{/each}
				</div>
			{/if}
		</div>
	{/if}

	{#if customPropertiesForType.length > 0}
		<div class="border-t border-[var(--color-border)] px-4 py-3">
			<h3 class="mb-2 text-[10px] uppercase tracking-wider text-[var(--color-muted)]">
				Properties
			</h3>
			<div class="grid gap-2">
				{#each customPropertiesForType as prop}
					{@const value = (detail.entity as Record<string, unknown>)[prop.key]}
					{@const warnings = warningsForKey(prop.key)}
					<div class="grid grid-cols-[120px_1fr_auto] items-start gap-3">
						<div class="flex flex-col text-xs">
							<span class="flex items-center gap-1 font-medium">
								<PropertyTypeIcon type={prop.type} class="text-[var(--color-muted)]" />
								<span>{prop.name}</span>
							</span>
							{#if prop.description}
								<span class="text-[10px] text-[var(--color-muted)]">{prop.description}</span>
							{/if}
						</div>
						<div class="flex min-w-0 flex-wrap items-center gap-2">
							{#if editingKey === prop.key}
								<div class="min-w-0 flex-1">
									<PropertyEditor {prop} bind:value={editingValue} />
								</div>
							{:else}
								<PropertyValue {prop} {value} />
							{/if}
							{#each warnings as w}
								<PropertyWarningChip warning={w} />
							{/each}
						</div>
						<div class="flex flex-shrink-0 gap-1 text-[10px]">
							{#if editingKey === prop.key}
								<button
									class="rounded border border-[var(--color-accent)] bg-[var(--color-accent)]/15 px-2 py-1 text-[var(--color-accent)] hover:bg-[var(--color-accent)]/25"
									onclick={commitEdit}
								>
									Save
								</button>
								<button
									class="rounded border border-[var(--color-border)] px-2 py-1 hover:bg-white/5"
									onclick={cancelEdit}
								>
									Cancel
								</button>
							{:else}
								<button
									class="rounded border border-[var(--color-border)] px-2 py-1 hover:bg-white/5"
									onclick={() => startEdit(prop.key, value)}
								>
									Edit
								</button>
							{/if}
						</div>
					</div>
				{/each}
			</div>
			<div class="mt-3 flex gap-3 text-[10px]">
				<button
					class="text-[var(--color-muted)] underline hover:text-[var(--color-text)]"
					onclick={() => (addingProperty = true)}
				>
					+ Add property
				</button>
				<a
					href="/settings/schema?type={detail.type}"
					class="text-[var(--color-muted)] hover:text-[var(--color-text)]"
				>
					Manage in schema editor →
				</a>
			</div>
		</div>
	{/if}

	{#if addingProperty}
		<PropertyConfigDialog
			entityType={detail.type}
			mode="add"
			existing={null}
			onCancel={() => (addingProperty = false)}
			onSaved={async () => {
				addingProperty = false;
				const fresh = await getEntity(detail.type, detail.file_id);
				detail.entity = fresh.entity;
				detail.value_warnings = fresh.value_warnings;
			}}
		/>
	{/if}

	{#if (detail.value_warnings ?? []).length > 0}
		{@const orphaned = (detail.value_warnings ?? []).filter((w) => w.kind === 'orphaned')}
		{#if orphaned.length > 0}
			<div class="border-t border-[var(--color-border)] px-4 py-3">
				<h3 class="mb-2 text-[10px] uppercase tracking-wider text-[var(--color-muted)]">
					Orphaned values
				</h3>
				<div class="grid gap-1">
					{#each orphaned as w}
						<div class="flex items-center gap-2 text-xs">
							<PropertyWarningChip warning={w} />
							<span class="text-[var(--color-muted)]">{w.key}</span>
						</div>
					{/each}
				</div>
				<p class="mt-1 text-[10px] text-[var(--color-muted)]">
					These keys aren't declared in the schema. Add them as a property in
					<a class="underline" href="/settings/schema?type={detail.type}">Schema</a>
					to use them, or edit the file to remove.
				</p>
			</div>
		{/if}
	{/if}

	{#if localBody.trim().length > 0}
		<div class="border-t border-[var(--color-border)] px-4 py-3">
			<BodyView body={localBody} fileId={detail.file_id} onChange={(newBody) => (localBody = newBody)} />
		</div>
	{:else}
		<div class="border-t border-[var(--color-border)] px-4 py-3 text-xs text-[var(--color-muted)]">
			No body yet.
			<button class="ml-1 underline hover:text-[var(--color-text)]" onclick={onEditBody}>
				Add one
			</button>
			or press
			<kbd class="rounded border border-[var(--color-border)] px-1">⇧E</kbd>.
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

	{#if threadInReplyTo || threadReplies.length > 0}
		<div class="border-t border-[var(--color-border)] px-4 py-3">
			<h3 class="mb-2 text-[10px] uppercase tracking-wider text-[var(--color-muted)]">Thread</h3>
			{#if threadInReplyTo}
				<div class="mb-2">
					<div class="mb-1 text-[10px] text-[var(--color-muted)]">In reply to</div>
					<button
						class="block w-full rounded border border-[var(--color-border)] bg-white/5 px-2 py-1 text-left text-xs hover:bg-white/10"
						onclick={() => goto(`/email/${threadInReplyTo!.file_id}`)}
					>
						<div class="font-medium">{threadInReplyTo.entity.subject as string}</div>
						<div class="text-[10px] text-[var(--color-muted)]">{threadInReplyTo.entity.date as string} · {threadInReplyTo.entity.direction as string}</div>
					</button>
				</div>
			{/if}
			{#if threadReplies.length > 0}
				<div class="mb-1 text-[10px] text-[var(--color-muted)]">Replies</div>
				<div class="grid gap-1.5">
					{#each threadReplies as reply}
						<button
							class="rounded border border-[var(--color-border)] bg-white/5 px-2 py-1 text-left text-xs hover:bg-white/10"
							onclick={() => goto(`/email/${reply.file_id}`)}
						>
							<div class="font-medium">{reply.entity.subject as string}</div>
							<div class="text-[10px] text-[var(--color-muted)]">{reply.entity.date as string} · {reply.entity.direction as string}</div>
						</button>
					{/each}
				</div>
			{/if}
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
						{bl.name ?? bl.src_file_id}
					</button>
				{/each}
			</div>
		</div>
	{/if}

	{#if children.length > 0}
		<div class="border-t border-[var(--color-border)] px-4 py-3">
			<h3 class="mb-2 text-[10px] uppercase tracking-wider text-[var(--color-muted)]">
				Sub-pages ({children.length})
			</h3>
			<div class="flex flex-col gap-1">
				{#each children as child (child.file_id)}
					<a
						class="flex items-center justify-between gap-2 rounded px-2 py-1 text-sm hover:bg-white/5"
						href="/{child.type}/{child.file_id}"
					>
						<span class="truncate">{child.name}</span>
						<span class="text-[10px] uppercase tracking-wider text-[var(--color-muted)]">
							{child.type}
						</span>
					</a>
				{/each}
			</div>
		</div>
	{/if}
</div>

{#if actionMenuOpen}
	<!-- Rendered outside the panel body and positioned `fixed` so the
		 nested overflow contexts (workspace grid, aside, panel scroll
		 region) can't clip it the way they did with the previous
		 `absolute` placement. -->
	<div
		role="menu"
		class="fixed z-50 w-44 overflow-hidden rounded border border-[var(--color-border)] bg-[var(--color-panel)] text-xs shadow-xl"
		style="top: {menuPos.top}px; right: {menuPos.right}px"
	>
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
