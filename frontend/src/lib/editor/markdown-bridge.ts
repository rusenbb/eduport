/**
 * Pre/post-processing so Tiptap + tiptap-markdown round-trip the
 * eduport-specific syntax cleanly:
 *
 *   - `[[wikilink]]` ↔ `<a class="wikilink" data-target="X">X</a>`
 *   - `![[view: name]]` (own line) ↔ `<div class="view-embed" data-view="name"></div>`
 *
 * tiptap-markdown speaks markdown-it on the way in and a serializer
 * registry on the way out. Rather than fight the registry for custom
 * nodes that don't have a native markdown equivalent, we lift them to
 * HTML at the boundary and let Tiptap's HTML parseHTML rules do the
 * work — markdown-it preserves inline/block HTML through the parse.
 */

const WIKILINK_RE = /\[\[([^\]\[]+)\]\]/g;
const VIEW_EMBED_LINE_RE = /^!\[\[view:\s*([^\]]+)\]\]\s*$/gm;

export function markdownToTiptapHtmlPreprocess(md: string): string {
	// View embeds first (block-level, anchored to their own line) so
	// the wikilink pass below doesn't accidentally grab the inner
	// `[[view: name]]` part.
	const withViews = md.replace(VIEW_EMBED_LINE_RE, (_m, name) => {
		const safe = String(name).trim().replace(/"/g, '&quot;');
		return `<div class="view-embed" data-view="${safe}"></div>`;
	});
	return withViews.replace(WIKILINK_RE, (_m, target) => {
		const safe = String(target).trim().replace(/"/g, '&quot;');
		return `<a class="wikilink" data-target="${safe}">${safe}</a>`;
	});
}

/**
 * Post-process tiptap-markdown's output. tiptap-markdown serializes
 * our custom HTML-ish nodes as raw HTML (because renderHTML emits
 * HTML); convert that back to the on-disk markdown syntax.
 */
export function tiptapMarkdownPostprocess(md: string): string {
	// Wikilinks: `<a class="wikilink" data-target="X">…</a>` → `[[X]]`
	// The text inside the anchor mirrors the target, so we ignore it.
	const withWikis = md.replace(
		/<a\s+[^>]*class="wikilink"[^>]*data-target="([^"]+)"[^>]*>[^<]*<\/a>/g,
		(_m, t) => `[[${decodeHtml(t)}]]`
	);
	// View embeds: `<div class="view-embed" data-view="X"></div>` → own line
	return withWikis
		.replace(
			/<div\s+[^>]*class="view-embed"[^>]*data-view="([^"]+)"[^>]*><\/div>/g,
			(_m, n) => `\n![[view: ${decodeHtml(n)}]]\n`
		)
		.replace(/\n{3,}/g, '\n\n')
		.trim()
		.concat('\n');
}

function decodeHtml(s: string): string {
	return s
		.replace(/&quot;/g, '"')
		.replace(/&amp;/g, '&')
		.replace(/&lt;/g, '<')
		.replace(/&gt;/g, '>');
}
