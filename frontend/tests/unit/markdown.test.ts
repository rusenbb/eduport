import { describe, expect, it } from 'vitest';
import { renderMarkdown } from '../../src/lib/markdown';

describe('renderMarkdown', () => {
	it('rewrites wikilinks to clickable anchors', () => {
		const { html } = renderMarkdown('See [[jane-doe-A4f2]] for details.');
		expect(html).toContain('<a class="wikilink');
		expect(html).toContain('data-target="jane-doe-A4f2"');
	});

	it('extracts checkboxes with line numbers', () => {
		const body = 'Plan:\n- [ ] task one\n- [x] task two';
		const { checkboxes } = renderMarkdown(body);
		expect(checkboxes).toEqual([
			{ line: 1, checked: false, text: 'task one' },
			{ line: 2, checked: true, text: 'task two' }
		]);
	});

	it('leaves non-wikilink content alone', () => {
		const { html } = renderMarkdown('# Heading\n\nplain paragraph');
		expect(html).toContain('<h1');
		expect(html).toContain('plain paragraph');
		expect(html).not.toContain('wikilink');
	});
});
