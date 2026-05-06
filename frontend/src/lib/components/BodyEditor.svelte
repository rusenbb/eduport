<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import type { EditorView } from '@codemirror/view';

	let { value = $bindable('') }: { value: string } = $props();

	let host: HTMLDivElement;
	let view: EditorView | null = null;
	let externalValue = '';

	onMount(async () => {
		const [{ markdown }, { oneDark }, { EditorState }, { EditorView, keymap }, { defaultKeymap, history, historyKeymap }] =
			await Promise.all([
				import('@codemirror/lang-markdown'),
				import('@codemirror/theme-one-dark'),
				import('@codemirror/state'),
				import('@codemirror/view'),
				import('@codemirror/commands')
			]);

		externalValue = value;
		view = new EditorView({
			parent: host,
			state: EditorState.create({
				doc: value,
				extensions: [
					history(),
					keymap.of([...defaultKeymap, ...historyKeymap]),
					markdown(),
					oneDark,
					EditorView.lineWrapping,
					EditorView.updateListener.of((update) => {
						if (update.docChanged) {
							externalValue = update.state.doc.toString();
							value = externalValue;
						}
					})
				]
			})
		});
	});

	$effect(() => {
		if (!view || value === externalValue) return;
		externalValue = value;
		view.dispatch({
			changes: { from: 0, to: view.state.doc.length, insert: value }
		});
	});

	onDestroy(() => {
		view?.destroy();
	});
</script>

<div
	bind:this={host}
	class="min-h-64 overflow-hidden rounded border border-[var(--color-border)] text-sm [&_.cm-editor]:min-h-64 [&_.cm-editor]:bg-[var(--color-bg)] [&_.cm-scroller]:font-mono"
></div>
