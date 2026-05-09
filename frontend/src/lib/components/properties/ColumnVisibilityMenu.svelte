<script lang="ts">
	import type { FieldDef } from '$lib/entities/meta';
	import type { Property } from '$lib/types/schema';
	import Icon from '../Icon.svelte';
	import PropertyTypeIcon from './PropertyTypeIcon.svelte';

	let {
		properties,
		builtinFields,
		visibleCustomKeys = $bindable([]),
		visibleBuiltinKeys = $bindable([]),
		onChange
	}: {
		properties: Property[];
		builtinFields: FieldDef[];
		visibleCustomKeys: string[];
		visibleBuiltinKeys: string[];
		onChange?: (next: { custom: string[]; builtin: string[] }) => void;
	} = $props();

	let open = $state(false);

	function toggleCustom(key: string) {
		const next = visibleCustomKeys.includes(key)
			? visibleCustomKeys.filter((k) => k !== key)
			: [...visibleCustomKeys, key];
		visibleCustomKeys = next;
		onChange?.({ custom: next, builtin: visibleBuiltinKeys });
	}

	function toggleBuiltin(key: string) {
		const next = visibleBuiltinKeys.includes(key)
			? visibleBuiltinKeys.filter((k) => k !== key)
			: [...visibleBuiltinKeys, key];
		visibleBuiltinKeys = next;
		onChange?.({ custom: visibleCustomKeys, builtin: next });
	}

	function close() {
		open = false;
	}
</script>

<div class="relative">
	<button
		class="rounded border border-[var(--color-border)] bg-white/5 px-2 py-1 text-xs hover:bg-white/10"
		onclick={() => (open = !open)}
	>
		Columns
	</button>
	{#if open}
		<div
			class="absolute right-0 top-full z-30 mt-1 w-64 overflow-hidden rounded border border-[var(--color-border)] bg-[var(--color-panel)] text-xs shadow-xl"
		>
			<div
				class="max-h-72 overflow-auto"
				role="menu"
				tabindex="-1"
				onmouseleave={() => {
					/* leave open until click elsewhere */
				}}
			>
				{#if builtinFields.length > 0}
					<div class="border-b border-[var(--color-border)] px-3 py-1 text-[10px] uppercase tracking-wider text-[var(--color-muted)]">
						Built-in
					</div>
					{#each builtinFields as field}
						<button
							class="flex w-full items-center gap-2 border-b border-[var(--color-border)] px-3 py-1.5 text-left last:border-b-0 hover:bg-white/5"
							onclick={() => toggleBuiltin(field.key)}
						>
							<input
								type="checkbox"
								checked={visibleBuiltinKeys.includes(field.key)}
								class="pointer-events-none"
							/>
							<span class="truncate">{field.label}</span>
						</button>
					{/each}
				{/if}
				{#if properties.length > 0}
					<div class="border-b border-[var(--color-border)] border-t px-3 py-1 text-[10px] uppercase tracking-wider text-[var(--color-muted)]">
						Custom
					</div>
					{#each properties as prop}
						<button
							class="flex w-full items-center gap-2 border-b border-[var(--color-border)] px-3 py-1.5 text-left last:border-b-0 hover:bg-white/5"
							onclick={() => toggleCustom(prop.key)}
						>
							<input
								type="checkbox"
								checked={visibleCustomKeys.includes(prop.key)}
								class="pointer-events-none"
							/>
							<PropertyTypeIcon type={prop.type} class="text-[var(--color-muted)]" />
							<span class="truncate">{prop.name}</span>
						</button>
					{/each}
				{/if}
			</div>
			<div class="border-t border-[var(--color-border)] px-3 py-1.5 text-right">
				<button class="text-[var(--color-muted)] hover:text-[var(--color-text)]" onclick={close}>
					<Icon name="x" size={12} />
				</button>
			</div>
		</div>
	{/if}
</div>
