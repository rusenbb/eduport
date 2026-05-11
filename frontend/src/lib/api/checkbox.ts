import { commands } from '../bindings';
import { unwrap } from './client';

export function toggleCheckbox(
	fileId: string,
	line: number,
	checked: boolean
): Promise<{ ok: boolean }> {
	return unwrap(commands.coreCheckboxToggle({ file_id: fileId, line, checked }));
}
