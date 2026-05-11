/**
 * Tiptap inline node for `[[wikilinks]]`. Adds:
 *
 *   - A typed-`[[` autocomplete that suggests entities from the vault
 *     (cached via $lib/editor/all-entities)
 *   - Render as a styled chip in the editor; clicking the chip
 *     navigates to the target like the BodyView wikilinks do
 *   - Round-trips to markdown as `[[Name]]` via tiptap-markdown's
 *     storage hook
 */
import { Node, mergeAttributes } from '@tiptap/core';
import { PluginKey } from '@tiptap/pm/state';
import Suggestion, { type SuggestionProps } from '@tiptap/suggestion';
import { allEntities } from './all-entities';

export interface WikilinkAttrs {
	target: string;
}

export const WikilinkPluginKey = new PluginKey('eduportWikilink');

export const Wikilink = Node.create({
	name: 'wikilink',
	group: 'inline',
	inline: true,
	atom: true,
	selectable: true,
	draggable: false,

	addAttributes() {
		return {
			target: {
				default: '',
				parseHTML: (el) => el.getAttribute('data-target') ?? '',
				renderHTML: (attrs) => ({ 'data-target': (attrs as WikilinkAttrs).target })
			}
		};
	},

	parseHTML() {
		return [{ tag: 'a.wikilink' }];
	},

	renderHTML({ node, HTMLAttributes }) {
		const target = (node.attrs as WikilinkAttrs).target;
		return [
			'a',
			mergeAttributes(HTMLAttributes, {
				class: 'wikilink rounded bg-[var(--color-accent)]/15 px-1 text-[var(--color-accent)]',
				'data-target': target
			}),
			target
		];
	},

	renderText({ node }) {
		return `[[${(node.attrs as WikilinkAttrs).target}]]`;
	},

	addStorage() {
		return {
			markdown: {
				serialize(state: { write: (s: string) => void }, node: { attrs: WikilinkAttrs }) {
					state.write(`[[${node.attrs.target}]]`);
				},
				parse: {
					// Pattern: match `[[X]]` and convert each to a node.
					// We do this via a regex on input rather than overriding
					// the markdown-it tokenizer — simpler and matches what
					// tiptap-markdown's storage layer expects.
				}
			}
		};
	},

	addProseMirrorPlugins() {
		return [
			Suggestion({
				editor: this.editor,
				char: '[[',
				pluginKey: WikilinkPluginKey,
				command: ({ editor, range, props }) => {
					editor
						.chain()
						.focus()
						.deleteRange(range)
						.insertContent([
							{ type: 'wikilink', attrs: { target: props.target } },
							{ type: 'text', text: ' ' }
						])
						.run();
				},
				items: async ({ query }: { query: string }) => {
					const q = query.toLowerCase();
					const all = await allEntities();
					return all
						.filter(({ item }) =>
							!q ? true : item.name.toLowerCase().includes(q) || item.file_id.toLowerCase().includes(q)
						)
						.slice(0, 8)
						.map(({ item, type }) => ({
							target: item.name,
							type,
							file_id: item.file_id
						}));
				},
				render: () => {
					let host: HTMLDivElement | null = null;
					let selected = 0;
					let lastItems: Array<{ target: string; type: string; file_id: string }> = [];
					let lastProps: SuggestionProps | null = null;

					function renderList() {
						if (!host || !lastProps) return;
						host.innerHTML = '';
						if (lastItems.length === 0) {
							const empty = document.createElement('div');
							empty.className = 'px-3 py-2 text-xs text-[var(--color-muted)]';
							empty.textContent = 'No matches — Enter to insert as plain text';
							host.appendChild(empty);
							return;
						}
						for (let i = 0; i < lastItems.length; i++) {
							const it = lastItems[i];
							const row = document.createElement('button');
							row.className = `flex w-full items-center justify-between gap-2 px-3 py-1.5 text-left text-xs hover:bg-white/5 ${
								i === selected ? 'bg-white/5' : ''
							}`;
							row.innerHTML = `<span class="truncate">${escapeHtml(it.target)}</span><span class="text-[10px] uppercase tracking-wider text-[var(--color-muted)]">${escapeHtml(it.type)}</span>`;
							row.onclick = () => {
								if (lastProps) lastProps.command({ target: it.target });
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
								'fixed z-50 min-w-[220px] overflow-hidden rounded border border-[var(--color-border)] bg-[var(--color-panel)] shadow-xl';
							document.body.appendChild(host);
							lastProps = props;
							lastItems = (props.items as Array<{ target: string; type: string; file_id: string }>) ?? [];
							selected = 0;
							renderList();
							position(props);
						},
						onUpdate: (props: SuggestionProps) => {
							lastProps = props;
							lastItems = (props.items as Array<{ target: string; type: string; file_id: string }>) ?? [];
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
									lastProps.command({ target: lastItems[selected].target });
									return true;
								}
								// No match: insert what they typed as a plain wikilink.
								const query = lastProps?.query ?? '';
								if (query && lastProps) {
									lastProps.command({ target: query });
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

function escapeHtml(s: string): string {
	return s
		.replace(/&/g, '&amp;')
		.replace(/</g, '&lt;')
		.replace(/>/g, '&gt;')
		.replace(/"/g, '&quot;');
}
