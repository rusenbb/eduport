<script lang="ts">
	let { tags = $bindable([]) }: { tags: string[] } = $props();

	let input = $state('');

	function commit() {
		const cleaned = input
			.split(',')
			.map((t) => t.trim())
			.filter((t) => t.length > 0 && !tags.includes(t));
		if (cleaned.length > 0) {
			tags = [...tags, ...cleaned];
		}
		input = '';
	}

	function onKey(event: KeyboardEvent) {
		if (event.key === 'Enter' || event.key === ',') {
			event.preventDefault();
			commit();
		} else if (event.key === 'Backspace' && input.length === 0 && tags.length > 0) {
			tags = tags.slice(0, -1);
		}
	}

	function remove(tag: string) {
		tags = tags.filter((t) => t !== tag);
	}
</script>

<div class="flex flex-wrap items-center gap-1.5 rounded border border-[var(--color-border)] bg-[var(--color-panel)] p-2">
	{#each tags as tag}
		<button
			type="button"
			class="rounded-full border border-[var(--color-accent)]/40 bg-[var(--color-accent)]/10 px-2 py-0.5 text-xs text-[var(--color-accent)] hover:bg-[var(--color-accent)]/20"
			onclick={() => remove(tag)}
		>
			{tag} ✕
		</button>
	{/each}
	<input
		bind:value={input}
		onkeydown={onKey}
		onblur={commit}
		placeholder="add tag…"
		class="flex-1 bg-transparent px-1 py-0.5 text-xs outline-none"
	/>
</div>
