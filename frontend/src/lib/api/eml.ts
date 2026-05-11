import { commands } from '../bindings';
import type { ParsedEml } from '../types';
import { unwrap } from './client';

/**
 * Send the raw .eml bytes through the Tauri command channel. Tauri
 * accepts `Vec<u8>` as a regular argument; we read the Blob into an
 * ArrayBuffer first, then pass it as a number array (the canonical
 * shape `tauri::command` expects for byte slices).
 */
export async function parseEml(
	file: File | Blob,
	_filename = 'message.eml'
): Promise<ParsedEml> {
	const buf = await file.arrayBuffer();
	const bytes = Array.from(new Uint8Array(buf));
	return unwrap(commands.coreParseEml(bytes)) as Promise<ParsedEml>;
}
