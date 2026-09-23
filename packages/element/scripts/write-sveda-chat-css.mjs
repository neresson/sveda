import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { buildSvedaChatCss } from '../src/scope-embed-css.mjs';

const root = dirname(fileURLToPath(import.meta.url));
const hostCss = readFileSync(resolve(root, '../src/host.css'), 'utf8');
const embedPath = resolve(root, '../../../apps/runtime/public/build/sveda/embed.css');
const outDir = resolve(root, '../../../apps/runtime/public/build/sveda');
const outPath = resolve(outDir, 'sveda-chat.css');

if (!existsSync(embedPath)) {
  console.error('sveda-chat.css: missing embed.css. Build the iframe embed first.');
  process.exit(1);
}

mkdirSync(outDir, { recursive: true });
writeFileSync(outPath, buildSvedaChatCss(hostCss, readFileSync(embedPath, 'utf8')));
