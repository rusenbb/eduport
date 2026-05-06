<script lang="ts">
	import type { EntityType } from '$lib/types';
	import WikilinkPicker from './WikilinkPicker.svelte';

	let {
		values = $bindable([]),
		type,
		onChange
	}: {
		values: string[];
		type: EntityType;
		onChange?: (values: string[]) => void;
	} = $props();

	function setValues(next: string[]) {
		values = next;
		onChange?.(next);
	}

	function add() {
		setValues([...values, '']);
	}

	function remove(index: number) {
		setValues(values.filter((_, i) => i !== index));
	}

	function updateAt(index: number, value: string) {
		setValues(values.map((current, i) => (i === index ? value : current)));
	}
</script>

<div class="grid gap-2">
	{#each values as value, index}
		<div class="flex gap-2">
			<div class="min-w-0 flex-1">
				<WikilinkPicker {value} {type} onChange={(next) => updateAt(index, next)} />
			</div>
			<button
				type="button"
				class="rounded border border-[var(--color-border)] px-2 text-xs hover:bg-white/5"
				onclick={() => remove(index)}
			>
				Remove
			</button>
		</div>
	{/each}
	<button
		type="button"
		class="w-fit rounded border border-[var(--color-border)] px-3 py-1.5 text-xs hover:bg-white/5"
		onclick={add}
	>
		Add link
	</button>
</div>
