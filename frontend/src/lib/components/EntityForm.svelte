<script lang="ts">
	import { goto } from '$app/navigation';
	import { createEntity, updateEntity } from '$lib/api/entities';
	import { FIELD_DEFS, TYPE_LABELS, typeTag, userTags, type FieldDef } from '$lib/entities/meta';
	import type { EntityType } from '$lib/types';
	import BodyEditor from './BodyEditor.svelte';
	import TagPicker from './TagPicker.svelte';
	import WikilinkListPicker from './WikilinkListPicker.svelte';
	import WikilinkPicker from './WikilinkPicker.svelte';

	let {
		type,
		fileId = null,
		initial = null,
		includeBody = true,
		onCancel,
		onDone
	}: {
		type: EntityType;
		fileId?: string | null;
		initial?: { frontmatter: Record<string, unknown>; body: string } | null;
		includeBody?: boolean;
		onCancel?: () => void;
		onDone?: (fileId: string) => void;
	} = $props();

	const defs = $derived(FIELD_DEFS[type]);

	function initialValue(def: FieldDef): unknown {
		const value = initial?.frontmatter?.[def.key];
		if (value !== undefined && value !== null) return value;
		if (def.kind === 'wikilinks') return [];
		if (def.kind === 'resources') return [];
		if (def.key === 'status' && type === 'application') return 'planning';
		if (def.key === 'direction' && type === 'email') return 'outbound';
		if (def.key === 'date' && type === 'email') return new Date().toISOString().slice(0, 10);
		return '';
	}

	function makeInitialFields() {
		return Object.fromEntries(defs.map((def) => [def.key, initialValue(def)]));
	}

	let name = $state('');
	let tags = $state<string[]>([]);
	let body = $state('');
	let fieldValues = $state<Record<string, any>>({});
	let saving = $state(false);
	let error: string | null = $state(null);
	let initialized = $state(false);

	$effect(() => {
		if (initialized) return;
		const fm = initial?.frontmatter ?? {};
		name = typeof fm.name === 'string' ? fm.name : '';
		tags = userTags(fm, type);
		body = initial?.body ?? '';
		fieldValues = makeInitialFields();
		initialized = true;
	});

	function stringValue(key: string): string {
		const value = fieldValues[key];
		if (Array.isArray(value)) return value.join(', ');
		return typeof value === 'string' ? value : value == null ? '' : String(value);
	}

	function setString(key: string, value: string) {
		fieldValues = { ...fieldValues, [key]: value };
	}

	function parseResource(value: unknown, key: string): unknown[] {
		if (Array.isArray(value)) return value;
		const raw = typeof value === 'string' ? value.trim() : '';
		if (!raw) return [];
		const parsed = JSON.parse(raw);
		if (!Array.isArray(parsed)) throw new Error(`${key} must be a JSON array`);
		return parsed;
	}

	function splitCsv(value: unknown): string[] {
		if (Array.isArray(value)) return value.map(String).filter(Boolean);
		return String(value ?? '')
			.split(',')
			.map((part) => part.trim())
			.filter(Boolean);
	}

	function buildFrontmatter(): Record<string, unknown> {
		const out: Record<string, unknown> = {
			tags: [typeTag(type), ...tags],
			name: name.trim()
		};
		for (const def of defs) {
			const value = fieldValues[def.key];
			if (def.kind === 'resources') {
				const parsed = parseResource(value, def.key);
				if (parsed.length) out[def.key] = parsed;
				continue;
			}
			if (def.kind === 'wikilinks') {
				const links = Array.isArray(value) ? value.filter(Boolean) : [];
				out[def.key] = links;
				continue;
			}
			if (type === 'email' && ['to', 'cc', 'bcc'].includes(def.key)) {
				out[def.key] = splitCsv(value);
				continue;
			}
			if (value === '' || value === null || value === undefined) continue;
			out[def.key] = value;
		}
		if (type === 'document' && !out.title) out.title = out.name;
		return out;
	}

	async function save() {
		error = null;
		let frontmatter: Record<string, unknown>;
		try {
			frontmatter = buildFrontmatter();
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
			return;
		}

		saving = true;
		try {
			let resultId: string;
			if (fileId) {
				const r = await updateEntity(type, fileId, frontmatter, body);
				resultId = r.file_id;
			} else {
				const r = await createEntity(type, frontmatter, body);
				resultId = r.file_id;
			}
			onDone?.(resultId);
			goto(`/${type}/${resultId}`);
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			saving = false;
		}
	}
</script>

<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
	<div class="flex w-[760px] max-w-[94vw] max-h-[92vh] flex-col overflow-hidden rounded-lg border border-[var(--color-border)] bg-[var(--color-panel)] shadow-2xl">
		<header class="border-b border-[var(--color-border)] px-5 py-3">
			<h2 class="text-lg font-semibold">{fileId ? 'Edit' : 'New'} {TYPE_LABELS[type]}</h2>
		</header>

		<div class="flex-1 overflow-auto px-5 py-4">
			<label class="block">
				<span class="block text-xs uppercase tracking-wider text-[var(--color-muted)]">Name</span>
				<input
					bind:value={name}
					class="mt-1 w-full rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-3 py-2 text-sm outline-none focus:border-[var(--color-accent)]"
					placeholder="Display name"
					required
				/>
			</label>

			<div class="mt-4">
				<span class="block text-xs uppercase tracking-wider text-[var(--color-muted)]">Tags</span>
				<div class="mt-1"><TagPicker bind:tags /></div>
			</div>

			{#if defs.length > 0}
				<div class="mt-5 grid gap-4 md:grid-cols-2">
					{#each defs as def}
						<label class={def.kind === 'resources' || def.kind === 'wikilinks' ? 'block md:col-span-2' : 'block'}>
							<span class="block text-xs uppercase tracking-wider text-[var(--color-muted)]">{def.label}</span>
							{#if def.kind === 'select'}
								<select
									value={stringValue(def.key)}
									onchange={(event) => setString(def.key, event.currentTarget.value)}
									class="mt-1 w-full rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-3 py-2 text-sm text-[var(--color-text)] outline-none focus:border-[var(--color-accent)]"
								>
									<option value="">Unset</option>
									{#each def.options ?? [] as option}
										<option value={option}>{option}</option>
									{/each}
								</select>
							{:else if def.kind === 'wikilink' && def.linkType}
								<div class="mt-1">
									<WikilinkPicker
										value={stringValue(def.key)}
										type={def.linkType}
										placeholder={`Select ${TYPE_LABELS[def.linkType]}`}
										onChange={(next) => setString(def.key, next)}
									/>
								</div>
							{:else if def.kind === 'wikilinks' && def.linkType}
								<div class="mt-1">
									<WikilinkListPicker
										values={Array.isArray(fieldValues[def.key]) ? fieldValues[def.key] : []}
										type={def.linkType}
										onChange={(next) => (fieldValues = { ...fieldValues, [def.key]: next })}
									/>
								</div>
							{:else if def.kind === 'resources'}
								<textarea
									value={JSON.stringify(fieldValues[def.key] ?? [], null, 2)}
									oninput={(event) => setString(def.key, event.currentTarget.value)}
									class="mt-1 h-24 w-full rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-3 py-2 font-mono text-xs outline-none focus:border-[var(--color-accent)]"
								></textarea>
							{:else}
								<input
									value={stringValue(def.key)}
									type={def.kind === 'date' ? 'date' : def.kind === 'email' ? 'email' : def.kind === 'url' ? 'url' : 'text'}
									placeholder={def.placeholder ?? ''}
									oninput={(event) => setString(def.key, event.currentTarget.value)}
									class="mt-1 w-full rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-3 py-2 text-sm outline-none focus:border-[var(--color-accent)]"
								/>
							{/if}
						</label>
					{/each}
				</div>
			{/if}

			{#if includeBody}
				<label class="mt-5 block">
					<span class="block text-xs uppercase tracking-wider text-[var(--color-muted)]">Body</span>
					<div class="mt-1">
						<BodyEditor bind:value={body} />
					</div>
				</label>
			{/if}

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
				disabled={!name.trim() || saving}
				onclick={save}
			>
				{saving ? 'Saving...' : 'Save'}
			</button>
		</footer>
	</div>
</div>
