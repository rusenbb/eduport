<script lang="ts">
	// Notes are entities of type 'note' under the same /entities/note route on the
	// sidecar. Redirect to the generic list page.
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';

	onMount(() => {
		goto('/note', { replaceState: true });
	});
</script>

<div class="p-8 text-center text-[var(--color-muted)]">Redirecting…</div>
