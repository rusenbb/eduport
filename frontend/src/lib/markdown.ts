import { marked } from 'marked';

const WIKILINK_RE = /\[\[([^\]\[]+)\]\]/g;
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

	const html = marked.parse(body, { async: false }) as string;
	const linked = html.replace(WIKILINK_RE, (_match, target) => {
		const safe = String(target).trim().replace(/"/g, '&quot;');
		return `<a class="wikilink text-[var(--color-accent)] hover:underline" data-target="${safe}">${safe}</a>`;
	});

	return { html: linked, checkboxes };
}

// Re-exported for tests
export const _internal = { WIKILINK_RE, CHECKBOX_RE };
