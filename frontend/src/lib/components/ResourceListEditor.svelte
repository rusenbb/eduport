<script lang="ts">
	// GUI editor for `Vec<LinkResource>` / `Vec<EmailResource>` fields
	// (the entity built-ins `links` / `emails`). Replaces the previous
	// raw-JSON textarea — gives label + value inputs per row plus an
	// add/remove affordance.
	//
	// Shape matches the Rust types in
	// `crates/eduport-core/src/entity/resources.rs`:
	//   LinkResource  { label, url }
	//   EmailResource { label, email, person? }
	// The optional `person` wikilink on EmailResource is preserved
	// verbatim — the editor doesn't surface it (no high-traffic UI
	// path uses it today) but it round-trips on save.

	import Icon from './Icon.svelte';

	type LinkItem = { label: string; url: string };
	type EmailItem = { label: string; email: string; person?: string | null };
	type AnyItem = Record<string, unknown>;

	let {
		mode,
		values = $bindable<AnyItem[]>([]),
		onChange
	}: {
		mode: 'link' | 'email';
		values: AnyItem[];
		onChange?: (next: AnyItem[]) => void;
	} = $props();

	const primaryKey = $derived(mode === 'link' ? 'url' : 'email');
	const primaryPlaceholder = $derived(mode === 'link' ? 'https://…' : 'name@example.com');
	const primaryType = $derived(mode === 'link' ? 'url' : 'email');

	function update(next: AnyItem[]) {
		values = next;
		onChange?.(next);
	}

	function addRow() {
		const blank: AnyItem =
			mode === 'link'
				? ({ label: '', url: '' } as LinkItem)
				: ({ label: '', email: '' } as EmailItem);
		update([...values, blank]);
	}

	function removeRow(index: number) {
		update(values.filter((_, i) => i !== index));
	}

	function setLabel(index: number, label: string) {
		update(values.map((item, i) => (i === index ? { ...item, label } : item)));
	}

	function setPrimary(index: number, primary: string) {
		update(values.map((item, i) => (i === index ? { ...item, [primaryKey]: primary } : item)));
	}

	function primaryValue(item: AnyItem): string {
		const v = item[primaryKey];
		return typeof v === 'string' ? v : '';
	}

	function labelValue(item: AnyItem): string {
		return typeof item.label === 'string' ? item.label : '';
	}
</script>

<div class="flex flex-col gap-1.5">
	{#each values as item, i (i)}
		<div class="flex items-center gap-1.5">
			<input
				type="text"
				value={labelValue(item)}
				placeholder="Label"
				oninput={(e) => setLabel(i, (e.currentTarget as HTMLInputElement).value)}
				class="w-32 rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-2 py-1 text-xs outline-none focus:border-[var(--color-accent)]"
			/>
			<input
				type={primaryType}
				value={primaryValue(item)}
				placeholder={primaryPlaceholder}
				oninput={(e) => setPrimary(i, (e.currentTarget as HTMLInputElement).value)}
				class="flex-1 rounded border border-[var(--color-border)] bg-[var(--color-bg)] px-2 py-1 text-xs outline-none focus:border-[var(--color-accent)]"
			/>
			<button
				type="button"
				class="text-[var(--color-muted)] hover:text-[var(--color-text)]"
				onclick={() => removeRow(i)}
				aria-label="Remove row"
			>
				<Icon name="x" size={14} />
			</button>
		</div>
	{/each}
	<button
		type="button"
		class="self-start rounded border border-dashed border-[var(--color-border)] px-2 py-1 text-xs text-[var(--color-muted)] hover:border-[var(--color-text)] hover:text-[var(--color-text)]"
		onclick={addRow}
	>
		<Icon name="plus" size={12} /> Add {mode === 'link' ? 'link' : 'email'}
	</button>
</div>
