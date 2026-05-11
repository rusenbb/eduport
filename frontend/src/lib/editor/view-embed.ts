/**
 * Tiptap block node for inline view embeds — `![[view: name]]`.
 *
 * Inserted via the slash menu; renders as a placeholder card in the
 * editor and as a clickable embedded card in BodyView (markdown
 * viewer). Markdown round-trip: serialize to `![[view: name]]`,
 * parse via a pre-pass in the editor's setContent path.
 */
import { Node, mergeAttributes } from '@tiptap/core';

export interface ViewEmbedAttrs {
	name: string;
}

export const ViewEmbed = Node.create({
	name: 'viewEmbed',
	group: 'block',
	atom: true,
	selectable: true,
	draggable: true,

	addAttributes() {
		return {
			name: {
				default: '',
				parseHTML: (el) => el.getAttribute('data-view') ?? '',
				renderHTML: (attrs) => ({ 'data-view': (attrs as ViewEmbedAttrs).name })
			}
		};
	},

	parseHTML() {
		return [{ tag: 'div.view-embed' }];
	},

	renderHTML({ node, HTMLAttributes }) {
		const name = (node.attrs as ViewEmbedAttrs).name;
		return [
			'div',
			mergeAttributes(HTMLAttributes, {
				class:
					'view-embed my-2 flex items-center gap-2 rounded border border-dashed border-[var(--color-border)] bg-white/5 px-3 py-2 text-sm',
				'data-view': name
			}),
			['span', {}, '📊'],
			['span', { class: 'font-medium' }, `View: ${name || '(unnamed)'}`]
		];
	},

	renderText({ node }) {
		return `![[view: ${(node.attrs as ViewEmbedAttrs).name}]]`;
	},

	addCommands() {
		return {
			insertViewEmbed:
				(name: string) =>
				({ commands }: { commands: { insertContent: (c: unknown) => boolean } }) =>
					commands.insertContent({
						type: 'viewEmbed',
						attrs: { name }
					})
		} as never;
	}
});

/**
 * Regex matching the on-disk embed syntax. Anchored to its own line
 * because the placeholder is block-level; we don't want to grab a
 * stray sequence in the middle of a paragraph.
 */
export const VIEW_EMBED_RE = /^!\[\[view:\s*([^\]]+)\]\]\s*$/;
