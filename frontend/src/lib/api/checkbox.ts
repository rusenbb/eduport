import { apiFetch } from './client';

export function toggleCheckbox(
	fileId: string,
	line: number,
	checked: boolean
): Promise<{ ok: true }> {
	return apiFetch('/checkbox/toggle', {
		method: 'POST',
		body: JSON.stringify({ file_id: fileId, line, checked })
	});
}
