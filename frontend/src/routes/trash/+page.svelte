<script lang="ts">
	// Trash UI is deferred until the sidecar grows /trash list + restore endpoints.
	// For now, point the user at the data folder via a friendly message.
	import { settings } from '$lib/stores/settings';
</script>

<main class="p-6">
	<h1 class="mb-4 text-2xl font-semibold">Trash</h1>
	<p class="text-sm text-[var(--color-muted)]">
		Deleted entities are moved to <code class="rounded bg-white/5 px-1 py-0.5">.eduport-trash/</code>
		inside your data folder. v1 doesn't have an in-app restore UI yet — open the folder in your
		file manager to restore manually.
	</p>
	{#if $settings}
		<p class="mt-3 text-xs text-[var(--color-muted)]">
			Path: <code>{$settings.data_folder}/.eduport-trash/</code>
		</p>
	{/if}
</main>
