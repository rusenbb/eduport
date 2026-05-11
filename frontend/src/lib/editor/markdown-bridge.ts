/**
 * Pre/post-processing so Tiptap + tiptap-markdown round-trip
 * `[[wikilinks]]` cleanly. Tiptap-markdown speaks markdown-it on the
 * way in and a serializer registry on the way out. Rather than fight
 * the registry for a custom inline node, we lift wikilinks to HTML at
 * the boundary and let Tiptap's parseHTML rules do the work —
 * markdown-it preserves inline HTML through the parse.
 */

const WIKILINK_RE = /\[\[([^\]\[]+)\]\]/g;

export function markdownToTiptapHtmlPreprocess(md: string): string {
	return md.replace(WIKILINK_RE, (_m, target) => {
		const safe = String(target).trim().replace(/"/g, '&quot;');
		return `<a class="wikilink" data-target="${safe}">${safe}</a>`;
	});
}

/**
 * Post-process tiptap-markdown's output. Tiptap-markdown serializes
 * our custom HTML-ish wikilink node as raw HTML (because renderHTML
 * emits HTML); convert that back to the on-disk `[[X]]` syntax.
 */
export function tiptapMarkdownPostprocess(md: string): string {
	return md
		.replace(
			/<a\s+[^>]*class="wikilink"[^>]*data-target="([^"]+)"[^>]*>[^<]*<\/a>/g,
			(_m, t) => `[[${decodeHtml(t)}]]`
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
