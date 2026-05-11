/**
 * Tiptap slash-command extension. Typed `/` opens a menu of block
 * inserts: heading 1/2/3, bullet list, numbered list, todo list,
 * quote, code block, divider, embed view.
 *
 * Generic — no eduport-specific commands. The same extension would
 * work in a future generic-Notion product.
 */
import { Extension, type Editor, type Range } from '@tiptap/core';
import Suggestion, { type SuggestionProps } from '@tiptap/suggestion';

interface SlashItem {
	label: string;
	description: string;
	run: (editor: Editor, range: Range) => void;
}

const ITEMS: SlashItem[] = [
	{
		label: 'Heading 1',
		description: 'Big section heading',
		run: (e, r) => e.chain().focus().deleteRange(r).setNode('heading', { level: 1 }).run()
	},
	{
		label: 'Heading 2',
		description: 'Medium section heading',
		run: (e, r) => e.chain().focus().deleteRange(r).setNode('heading', { level: 2 }).run()
	},
	{
		label: 'Heading 3',
		description: 'Small section heading',
		run: (e, r) => e.chain().focus().deleteRange(r).setNode('heading', { level: 3 }).run()
	},
	{
		label: 'Bulleted list',
		description: 'Plain unordered list',
		run: (e, r) => e.chain().focus().deleteRange(r).toggleBulletList().run()
	},
	{
		label: 'Numbered list',
		description: '1, 2, 3 ordered list',
		run: (e, r) => e.chain().focus().deleteRange(r).toggleOrderedList().run()
	},
	{
		label: 'To-do list',
		description: 'Checkboxes that round-trip to markdown',
		run: (e, r) => e.chain().focus().deleteRange(r).toggleTaskList().run()
	},
	{
		label: 'Quote',
		description: 'Indented block quote',
		run: (e, r) => e.chain().focus().deleteRange(r).toggleBlockquote().run()
	},
	{
		label: 'Code block',
		description: 'Monospace fenced code',
		run: (e, r) => e.chain().focus().deleteRange(r).toggleCodeBlock().run()
	},
	{
		label: 'Divider',
		description: 'Horizontal rule',
		run: (e, r) => e.chain().focus().deleteRange(r).setHorizontalRule().run()
	},
	{
		label: 'Embed view',
		description: 'Insert ![[view: name]] placeholder',
		run: (e, r) => {
			const name = window.prompt('Saved view name (from the view tabs):', '');
			if (!name) return;
			e.chain()
				.focus()
				.deleteRange(r)
				.insertContent({ type: 'viewEmbed', attrs: { name: name.trim() } })
				.run();
		}
	}
];

export const SlashMenu = Extension.create({
	name: 'slashMenu',

	addProseMirrorPlugins() {
		return [
			Suggestion({
				editor: this.editor,
				char: '/',
				startOfLine: false,
				pluginKey: { key: 'slash-menu' } as never,
				command: ({ editor, range, props }) => {
					const item = props as SlashItem;
					item.run(editor, range);
				},
				items: ({ query }: { query: string }) => {
					const q = query.toLowerCase();
					return ITEMS.filter(
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
						for (let i = 0; i < lastItems.length; i++) {
							const it = lastItems[i];
							const row = document.createElement('button');
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
								'fixed z-50 min-w-[260px] max-h-[300px] overflow-auto rounded border border-[var(--color-border)] bg-[var(--color-panel)] shadow-xl';
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
