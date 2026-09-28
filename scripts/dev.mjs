import { spawn } from 'node:child_process';
import { homedir, platform } from 'node:os';
import { join, resolve } from 'node:path';

const root = resolve(import.meta.dirname, '..');
const dataBase = platform() === 'darwin'
  ? join(homedir(), 'Library', 'Application Support')
  : platform() === 'win32'
    ? process.env.APPDATA ?? join(homedir(), 'AppData', 'Roaming')
    : process.env.XDG_DATA_HOME ?? join(homedir(), '.local', 'share');
const dbPath = join(dataBase, 'io.github.rusenbb.eduport', 'eduport.sqlite');
const cargoDir = join(root, 'src-tauri');
const cargoEnv = { ...process.env, EDUPORT_DB_PATH: dbPath };
const children = [];
let stopping = false;

function run(command, args, options = {}) {
  const child = spawn(command, args, { stdio: 'inherit', ...options });
  children.push(child);
  return child;
}

function waitForExit(child) {
  return new Promise((resolve, reject) => {
    child.once('error', reject);
    child.once('exit', (code, signal) => code === 0 || stopping
      ? resolve()
      : reject(new Error(`Process exited with ${signal ?? `code ${code}`}`)));
  });
}

async function waitForApi(child) {
  for (let attempt = 0; attempt < 120; attempt += 1) {
    if (child.exitCode !== null || child.signalCode !== null) throw new Error('Local SQLite API stopped during startup.');
    try {
      const response = await fetch('http://127.0.0.1:1422/health', { signal: AbortSignal.timeout(500) });
      if (response.ok) return;
    } catch { /* still starting */ }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error('Local SQLite API did not become ready.');
}

function shutdown() {
  stopping = true;
  for (const child of children) if (child.exitCode === null && child.signalCode === null) child.kill('SIGTERM');
}

process.on('SIGINT', shutdown);
process.on('SIGTERM', shutdown);

try {
  let apiRunning = false;
  try {
    apiRunning = (await fetch('http://127.0.0.1:1422/health', { signal: AbortSignal.timeout(500) })).ok;
  } catch { /* port is available */ }
  if (apiRunning) throw new Error('Eduport local API is already running on port 1422. Stop the earlier development server first.');
  console.log('Building local SQLite API…');
  await waitForExit(run('cargo', ['build', '--example', 'dev_api'], { cwd: cargoDir, env: cargoEnv }));
  const api = run(join(cargoDir, 'target', 'debug', 'examples', platform() === 'win32' ? 'dev_api.exe' : 'dev_api'), [], { cwd: cargoDir, env: cargoEnv });
  await waitForApi(api);
  const vite = run(process.execPath, [join(root, 'node_modules', 'vite', 'bin', 'vite.js'), 'dev', '--host', '127.0.0.1'], { cwd: root });
  await waitForExit(vite);
} catch (error) {
  console.error(error);
  process.exitCode = 1;
} finally {
  shutdown();
}
