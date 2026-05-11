import { marked } from 'marked';

const WIKILINK_RE = /\[\[([^\]\[]+)\]\]/g;
const VIEW_EMBED_LINE_RE = /^!\[\[view:\s*([^\]]+)\]\]\s*$/gm;
const CHECKBOX_RE = /^- \[( |x|X)\] (.+)$/gm;

export interface ParsedCheckbox {
	line: number; // line index within the body
	checked: boolean;
	text: string;
}

export interface RenderResult {
	html: string;
	checkboxes: ParsedCheckbox[];
}

/**
 * Render a markdown body with two extra behaviours layered on top of marked:
 * 1. `[[target]]` becomes `<a class="wikilink" data-target="target">target</a>`.
 * 2. The list of `- [ ]` / `- [x]` lines is extracted alongside the rendered HTML
 *    so the UI can render them as interactive checkboxes that map back to the
 *    original line number for the toggle endpoint.
 */
export function renderMarkdown(body: string): RenderResult {
	const checkboxes: ParsedCheckbox[] = [];
	body.split('\n').forEach((line, idx) => {
		const m = /^- \[( |x|X)\] (.+)$/.exec(line);
		if (m) {
			checkboxes.push({
				line: idx,
				checked: m[1].toLowerCase() === 'x',
				text: m[2].trim()
			});
		}
	});

	// `breaks: true` turns single newlines into <br> instead of
	// collapsing them to whitespace (CommonMark default). Obsidian
	// renders this way, and the user authors in an Obsidian-style
	// vault — without this, prose typed with newline breaks looks
	// like one run-on paragraph.
	// Substitute view embeds BEFORE handing off to marked. The
	// `![[view: name]]` line shape would otherwise look like an image
	// reference to marked and silently disappear.
	const stamped = body.replace(VIEW_EMBED_LINE_RE, (_m, name) => {
		const safe = String(name).trim().replace(/"/g, '&quot;');
		return `<div class="view-embed" data-view="${safe}"></div>`;
	});

	const html = marked.parse(stamped, { async: false, breaks: true, gfm: true }) as string;
	const linked = html.replace(WIKILINK_RE, (_match, target) => {
		const safe = String(target).trim().replace(/"/g, '&quot;');
		return `<a class="wikilink text-[var(--color-accent)] hover:underline" data-target="${safe}">${safe}</a>`;
	});

	return { html: linked, checkboxes };
}

// Re-exported for tests
export const _internal = { WIKILINK_RE, CHECKBOX_RE };
