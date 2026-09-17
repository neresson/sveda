import { copyFileSync, existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = dirname(fileURLToPath(import.meta.url));
const runtimeDir = resolve(root, '../../../apps/runtime/public/build/sveda');
const distDir = resolve(root, '../dist');

mkdirSync(distDir, { recursive: true });

for (const file of ['sveda-chat.js', 'sveda-chat.css']) {
  const src = resolve(runtimeDir, file);
  if (!existsSync(src)) {
    console.error(`copy-npm-dist: missing ${file} in ${runtimeDir}`);
    process.exit(1);
  }
  copyFileSync(src, resolve(distDir, file));
}

writeFileSync(
  resolve(distDir, 'index.d.ts'),
  `export declare const ELEMENT_TAG: "sveda-chat";

export declare function defineSvedaChatElement(): void;

export interface SvedaSessionPayload {
  origin: string;
  token: string;
  appearance?: Record<string, unknown> | null;
}

export declare function parseSessionAttributes(element: HTMLElement): {
  session: string | null;
  origin: string | null;
  token: string | null;
};

export declare function requestHostSession(sessionUrl: string): Promise<SvedaSessionPayload | null>;
export declare function resolveSvedaSession(element: HTMLElement): Promise<SvedaSessionPayload | null>;
`,
);
