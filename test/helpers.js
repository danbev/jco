import { execArgv } from 'node:process';
import { spawn } from 'node:child_process';

export const dev = execArgv.includes('--conditions=test');
export const jcoPath = dev ? 'src/jco.js' : 'dist/cli.mjs';

export async function exec (cmd, ...args) {
  let stdout = '', stderr = '';
  await new Promise((resolve, reject) => {
    const cp = spawn(cmd, args, { stdio: 'pipe' });
    cp.stdout.on('data', chunk => {
      stdout += chunk;
    });
    cp.stderr.on('data', chunk => {
      stderr += chunk;
    });
    cp.on('error', reject);
    cp.on('exit', code => code === 0 ? resolve() : reject(new Error(stderr || stdout)));
  });
  return { stdout, stderr };
}
