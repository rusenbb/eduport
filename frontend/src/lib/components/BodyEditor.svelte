<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { Editor } from '@tiptap/core';
	import StarterKit from '@tiptap/starter-kit';
	import TaskList from '@tiptap/extension-task-list';
	import TaskItem from '@tiptap/extension-task-item';
	import Link from '@tiptap/extension-link';
	import Placeholder from '@tiptap/extension-placeholder';
	import { Markdown } from 'tiptap-markdown';
	import { Wikilink } from '$lib/editor/wikilink';
	import { ViewEmbed } from '$lib/editor/view-embed';
	import { SlashMenu } from '$lib/editor/slash-menu';
	import {
		markdownToTiptapHtmlPreprocess,
		tiptapMarkdownPostprocess
	} from '$lib/editor/markdown-bridge';

	let { value = $bindable('') }: { value: string } = $props();

	let host: HTMLDivElement;
	let editor: Editor | null = null;
	let externalValue = '';

	onMount(() => {
		externalValue = value;
		editor = new Editor({
			element: host,
			extensions: [
				StarterKit.configure({
					// We provide our own Link via @tiptap/extension-link with
					// stricter config, so disable StarterKit's bundled link.
					link: false
				}),
				TaskList,
				TaskItem.configure({ nested: true }),
				Link.configure({
					openOnClick: false,
					HTMLAttributes: { class: 'text-[var(--color-accent)] underline' }
				}),
				Placeholder.configure({ placeholder: "Start writing — type '/' for blocks" }),
				Markdown.configure({
					html: true,
					linkify: false,
					transformPastedText: true,
					transformCopiedText: true
				}),
				Wikilink,
				ViewEmbed,
				SlashMenu
			],
			content: markdownToTiptapHtmlPreprocess(value),
			editorProps: {
				attributes: {
					class:
						'min-h-64 rounded border border-[var(--color-border)] bg-[var(--color-bg)] p-4 text-base focus:outline-none'
				}
			},
			onUpdate: ({ editor: ed }) => {
				const storage = (ed.storage as { markdown?: { getMarkdown?: () => string } }).markdown;
				const raw = storage?.getMarkdown ? storage.getMarkdown() : ed.getText();
				externalValue = tiptapMarkdownPostprocess(raw);
				value = externalValue;
			}
		});
	});

	$effect(() => {
		if (!editor || value === externalValue) return;
		externalValue = value;
		editor.commands.setContent(markdownToTiptapHtmlPreprocess(value), { emitUpdate: false });
	});

	onDestroy(() => {
		editor?.destroy();
		editor = null;
	});
</script>

<div
	bind:this={host}
	class="min-h-64 text-sm [&_.ProseMirror]:min-h-64 [&_.ProseMirror]:outline-none"
></div>
