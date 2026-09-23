import { appendFileSync, existsSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const scopedCss = resolve(root, 'dist/vue.css');
const stylesCss = resolve(root, 'dist/styles.css');

if (!existsSync(scopedCss)) {
  console.error(
    'append-scoped-css: dist/vue.css is missing after vite build. Scoped SFC CSS must be present.',
  );
  process.exit(1);
}

if (!existsSync(stylesCss)) {
  console.error('append-scoped-css: dist/styles.css is missing. Run build:css first.');
  process.exit(1);
}

const scoped = readFileSync(scopedCss, 'utf8').trim();
if (!scoped) {
  console.error('append-scoped-css: dist/vue.css is empty.');
  process.exit(1);
}

appendFileSync(stylesCss, `\n${scoped}\n`);
console.log('append-scoped-css: appended dist/vue.css onto dist/styles.css');
