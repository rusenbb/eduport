<script lang="ts">
	import type { ValueWarning } from '$lib/types/schema';
	import Icon from '../Icon.svelte';

	let { warning }: { warning: ValueWarning } = $props();

	const labelByKind: Record<ValueWarning['kind'], string> = {
		orphaned: 'orphan',
		type_mismatch: 'wrong type',
		out_of_options: 'off-list',
		broken_link: 'broken link',
		wrong_target_type: 'wrong target',
		required_missing: 'required'
	};
</script>

<span
	class="inline-flex items-center gap-1 rounded border border-yellow-500/40 bg-yellow-500/10 px-1.5 py-0.5 text-[10px] text-yellow-100"
	title={warning.message}
>
	<Icon name="warning" size={10} />
	<span>{labelByKind[warning.kind]}</span>
</span>
