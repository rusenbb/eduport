<script lang="ts">
	// Notes are entities of type 'note' under the generic /[type] route.
	// Keep `/notes` as a friendly alias and redirect to the canonical path.
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';

	onMount(() => {
		goto('/note', { replaceState: true });
	});
</script>

<div class="p-8 text-center text-[var(--color-muted)]">Redirecting…</div>
