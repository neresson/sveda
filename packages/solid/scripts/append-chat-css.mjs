import { appendFileSync, existsSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const chatCss = resolve(root, 'src/chat.css');
const stylesCss = resolve(root, 'dist/styles.css');

if (!existsSync(stylesCss)) {
  console.error('append-chat-css: dist/styles.css is missing. Run build:css first.');
  process.exit(1);
}

if (!existsSync(chatCss)) {
  console.error('append-chat-css: src/chat.css is missing.');
  process.exit(1);
}

const scoped = readFileSync(chatCss, 'utf8').trim();
if (!scoped) {
  console.error('append-chat-css: src/chat.css is empty.');
  process.exit(1);
}

appendFileSync(stylesCss, `\n${scoped}\n`);
console.log('append-chat-css: appended src/chat.css onto dist/styles.css');
