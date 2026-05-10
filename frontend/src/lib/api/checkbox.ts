import { coreInvoke } from './client';

export function toggleCheckbox(
	fileId: string,
	line: number,
	checked: boolean
): Promise<{ ok: boolean }> {
	return coreInvoke('core_checkbox_toggle', {
		body: { file_id: fileId, line, checked }
	});
}
