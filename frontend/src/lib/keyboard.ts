// Keyboard-shortcut helpers shared between the global handler in the
// root layout and the per-workspace handler in EntityWorkspace.
//
// Two responsibilities:
//   1. Detect whether the keydown target is a text-entry surface (so
//      we don't hijack the user's typing).
//   2. Render the modifier key labels per platform so help text and
//      kbd hints agree with what the user actually has to press
//      (⌘ on macOS, Ctrl elsewhere).

export function isTypingTarget(target: EventTarget | null): boolean {
	if (!(target instanceof HTMLElement)) return false;
	if (target.isContentEditable) return true;
	const tag = target.tagName;
	if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return true;
	return false;
}

export const isMac =
	typeof navigator !== 'undefined' &&
	/Mac|iPhone|iPad|iPod/.test(navigator.platform || navigator.userAgent || '');

export const modKey = isMac ? '⌘' : 'Ctrl';

/** Format a shortcut for display: `mod K` → "⌘K" / "Ctrl+K". */
export function formatShortcut(parts: string[]): string {
	const sep = isMac ? '' : '+';
	return parts
		.map((p) => {
			if (p === 'mod') return modKey;
			if (p === 'shift') return isMac ? '⇧' : 'Shift';
			if (p === 'alt') return isMac ? '⌥' : 'Alt';
			if (p === 'enter') return isMac ? '↵' : 'Enter';
			if (p === 'esc') return 'Esc';
			if (p === 'up') return '↑';
			if (p === 'down') return '↓';
			if (p === 'backspace') return isMac ? '⌫' : 'Backspace';
			return p;
		})
		.join(sep);
}
