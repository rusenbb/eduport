#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const repo = join(dirname(fileURLToPath(import.meta.url)), '..');
const candidates =
	process.platform === 'win32'
		? [
				['py', ['-3']],
				['python', []],
				['python3', []]
			]
		: [
				['python3', []],
				['python', []]
			];

let python = null;
for (const [cmd, args] of candidates) {
	const check = spawnSync(cmd, [...args, '--version'], { cwd: repo, stdio: 'ignore' });
	if (check.status === 0) {
		python = [cmd, args];
		break;
	}
}

if (!python) {
	console.error('Python 3 is required to build the desktop prerequisites.');
	process.exit(1);
}

const [cmd, args] = python;
const result = spawnSync(cmd, [...args, 'scripts/build_tauri_prereqs.py'], {
	cwd: repo,
	stdio: 'inherit'
});
process.exit(result.status ?? 1);
