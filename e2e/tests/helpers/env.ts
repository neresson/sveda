import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '../../..');

export function readDotenvValue(name: string, file = join(root, 'apps/runtime/.env')): string {
  const fromEnv = process.env[name]?.trim();
  if (fromEnv) {
    return fromEnv;
  }

  try {
    for (const raw of readFileSync(file, 'utf8').split('\n')) {
      const line = raw.trim();
      if (!line || line.startsWith('#')) {
        continue;
      }
      const stripped = line.startsWith('export ') ? line.slice(7).trim() : line;
      const eq = stripped.indexOf('=');
      if (eq <= 0) {
        continue;
      }
      const key = stripped.slice(0, eq).trim();
      if (key !== name) {
        continue;
      }
      let value = stripped.slice(eq + 1).trim();
      if (
        (value.startsWith('"') && value.endsWith('"')) ||
        (value.startsWith("'") && value.endsWith("'"))
      ) {
        value = value.slice(1, -1);
      }
      return value.trim();
    }
  } catch {
    return '';
  }

  return '';
}

export function hasDeepseekKey(): boolean {
  return Boolean(readDotenvValue('DEEPSEEK_API_KEY') || readDotenvValue('SVEDA_DEEPSEEK_API_KEY'));
}

export function applyLiveE2eEnv(): void {
  const key = readDotenvValue('DEEPSEEK_API_KEY') || readDotenvValue('SVEDA_DEEPSEEK_API_KEY');
  if (key && !process.env.DEEPSEEK_API_KEY) {
    process.env.DEEPSEEK_API_KEY = key;
  }
}

export type E2eTier = 'hermetic' | 'persistent' | 'live';

export function e2eTier(): E2eTier {
  if (hasDeepseekKey()) {
    return 'live';
  }
  if (process.env.SVEDA_E2E_PERSISTENT === '1') {
    return 'persistent';
  }
  return 'hermetic';
}
