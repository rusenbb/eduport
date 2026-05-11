/**
 * Tiptap slash-command extension. Typed `/` opens a menu with two
 * groups of items:
 *
 *   1. Block transforms — Heading 1/2/3, bullet/numbered/todo list,
 *      quote, code block, divider. Wraps StarterKit + TaskList
 *      commands.
 *   2. View embeds — one entry per **saved view** in the vault
 *      (pulled from `viewsStore`), pre-bound so selecting one
 *      inserts the `![[view: name]]` block directly. No second
 *      prompt, no typing the view name.
 *
 * Generic on the block-transform side; the view-embed entries are
 * eduport-specific only in that they come from eduport's saved-view
 * store. A future generic-Notion product would swap that source.
 */
import { Extension, type Editor, type Range } from '@tiptap/core';
import { PluginKey } from '@tiptap/pm/state';
import Suggestion, { type SuggestionProps } from '@tiptap/suggestion';
import { get } from 'svelte/store';
import { viewsStore } from '$lib/stores/views';

const SlashMenuPluginKey = new PluginKey('eduportSlashMenu');

interface SlashItem {
	label: string;
	description: string;
	group: 'block' | 'view';
	run: (editor: Editor, range: Range) => void;
}

const BLOCK_ITEMS: SlashItem[] = [
	{
		label: 'Heading 1',
		description: 'Big section heading',
		group: 'block',
		run: (e, r) => e.chain().focus().deleteRange(r).setNode('heading', { level: 1 }).run()
	},
	{
		label: 'Heading 2',
		description: 'Medium section heading',
		group: 'block',
		run: (e, r) => e.chain().focus().deleteRange(r).setNode('heading', { level: 2 }).run()
	},
	{
		label: 'Heading 3',
		description: 'Small section heading',
		group: 'block',
		run: (e, r) => e.chain().focus().deleteRange(r).setNode('heading', { level: 3 }).run()
	},
	{
		label: 'Bulleted list',
		description: 'Plain unordered list',
		group: 'block',
		run: (e, r) => e.chain().focus().deleteRange(r).toggleBulletList().run()
	},
	{
		label: 'Numbered list',
		description: '1, 2, 3 ordered list',
		group: 'block',
		run: (e, r) => e.chain().focus().deleteRange(r).toggleOrderedList().run()
	},
	{
		label: 'To-do list',
		description: 'Checkboxes that round-trip to markdown',
		group: 'block',
		run: (e, r) => e.chain().focus().deleteRange(r).toggleTaskList().run()
	},
	{
		label: 'Quote',
		description: 'Indented block quote',
		group: 'block',
		run: (e, r) => e.chain().focus().deleteRange(r).toggleBlockquote().run()
	},
	{
		label: 'Code block',
		description: 'Monospace fenced code',
		group: 'block',
		run: (e, r) => e.chain().focus().deleteRange(r).toggleCodeBlock().run()
	},
	{
		label: 'Divider',
		description: 'Horizontal rule',
		group: 'block',
		run: (e, r) => e.chain().focus().deleteRange(r).setHorizontalRule().run()
	}
];

function viewItems(): SlashItem[] {
	const file = get(viewsStore).file;
	if (!file) return [];
	const items: SlashItem[] = [];
	for (const [type, typeViews] of Object.entries(file.types)) {
		for (const view of typeViews.views) {
			items.push({
				label: `Embed view · ${view.name}`,
				description: `${type} · live list of matching entities`,
				group: 'view',
				run: (e, r) =>
					e
						.chain()
						.focus()
						.deleteRange(r)
						.insertContent({ type: 'viewEmbed', attrs: { name: view.name } })
						.run()
			});
		}
	}
	return items;
}

function allItems(): SlashItem[] {
	return [...BLOCK_ITEMS, ...viewItems()];
}

export const SlashMenu = Extension.create({
	name: 'slashMenu',

	addProseMirrorPlugins() {
		return [
			Suggestion({
				editor: this.editor,
				char: '/',
				startOfLine: false,
				pluginKey: SlashMenuPluginKey,
				command: ({ editor, range, props }) => {
					const item = props as SlashItem;
					item.run(editor, range);
				},
				items: ({ query }: { query: string }) => {
					const q = query.toLowerCase();
					return allItems().filter(
						(it) =>
							!q ||
							it.label.toLowerCase().includes(q) ||
							it.description.toLowerCase().includes(q)
					);
				},
				render: () => {
					let host: HTMLDivElement | null = null;
					let selected = 0;
					let lastItems: SlashItem[] = [];
					let lastProps: SuggestionProps | null = null;

					function renderList() {
						if (!host || !lastProps) return;
						host.innerHTML = '';
						if (lastItems.length === 0) {
							const empty = document.createElement('div');
							empty.className = 'px-3 py-2 text-xs text-[var(--color-muted)]';
							empty.textContent = 'No commands match';
							host.appendChild(empty);
							return;
						}
						let lastGroup: SlashItem['group'] | null = null;
						for (let i = 0; i < lastItems.length; i++) {
							const it = lastItems[i];
							if (it.group !== lastGroup) {
								const sep = document.createElement('div');
								sep.className =
									'px-3 pt-2 pb-1 text-[10px] uppercase tracking-wider text-[var(--color-muted)]';
								sep.textContent = it.group === 'block' ? 'Blocks' : 'Embeds';
								host.appendChild(sep);
								lastGroup = it.group;
							}
							const row = document.createElement('button');
							row.dataset.slashRow = String(i);
							row.className = `flex w-full flex-col items-start gap-0.5 px-3 py-2 text-left hover:bg-white/5 ${
								i === selected ? 'bg-white/5' : ''
							}`;
							const title = document.createElement('div');
							title.className = 'text-sm';
							title.textContent = it.label;
							const desc = document.createElement('div');
							desc.className = 'text-[10px] text-[var(--color-muted)]';
							desc.textContent = it.description;
							row.appendChild(title);
							row.appendChild(desc);
							row.onclick = () => {
								if (lastProps) lastProps.command(it);
							};
							host.appendChild(row);
						}
						// Keep the highlighted row visible inside the popup
						// when the user navigates past the bottom edge.
						const target = host.querySelector(
							`[data-slash-row="${selected}"]`
						) as HTMLElement | null;
						target?.scrollIntoView({ block: 'nearest' });
					}

					function position(props: SuggestionProps) {
						if (!host || !props.clientRect) return;
						const rect = props.clientRect();
						if (!rect) return;
						host.style.left = `${rect.left}px`;
						host.style.top = `${rect.bottom + 4}px`;
					}

					return {
						onStart: (props: SuggestionProps) => {
							host = document.createElement('div');
							host.className =
								'fixed z-50 min-w-[260px] max-h-[320px] overflow-auto rounded border border-[var(--color-border)] bg-[var(--color-panel)] shadow-xl';
							document.body.appendChild(host);
							lastProps = props;
							lastItems = (props.items as SlashItem[]) ?? [];
							selected = 0;
							renderList();
							position(props);
						},
						onUpdate: (props: SuggestionProps) => {
							lastProps = props;
							lastItems = (props.items as SlashItem[]) ?? [];
							if (selected >= lastItems.length) selected = Math.max(0, lastItems.length - 1);
							renderList();
							position(props);
						},
						onKeyDown: (props: { event: KeyboardEvent }) => {
							const e = props.event;
							if (e.key === 'ArrowDown') {
								selected = (selected + 1) % Math.max(1, lastItems.length);
								renderList();
								return true;
							}
							if (e.key === 'ArrowUp') {
								selected = (selected - 1 + lastItems.length) % Math.max(1, lastItems.length);
								renderList();
								return true;
							}
							if (e.key === 'Enter') {
								if (lastItems.length > 0 && lastProps) {
									lastProps.command(lastItems[selected]);
									return true;
								}
							}
							if (e.key === 'Escape') {
								host?.remove();
								host = null;
								return true;
							}
							return false;
						},
						onExit: () => {
							host?.remove();
							host = null;
						}
					};
				}
			})
		];
	}
});
