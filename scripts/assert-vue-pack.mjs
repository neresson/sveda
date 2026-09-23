#!/usr/bin/env node
/**
 * Packs @sveda-ai/vue with pnpm (applies publishConfig + workspace:* rewrite)
 * and asserts the packed package.json is consumer-ready.
 */
import { execFileSync } from 'node:child_process';
import { mkdtempSync, readdirSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const vueDir = join(root, 'packages/vue');

function fail(msg) {
  console.error(`assert-vue-pack: ${msg}`);
  process.exit(1);
}

function isSemverRange(value) {
  if (typeof value !== 'string' || !value) return false;
  if (value === 'workspace:*' || value.startsWith('workspace:')) return false;
  return true;
}

const outDir = mkdtempSync(join(tmpdir(), 'sveda-vue-pack-'));

try {
  execFileSync('pnpm', ['pack', '--pack-destination', outDir], {
    cwd: vueDir,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  });

  const tarballs = readdirSync(outDir).filter((f) => f.endsWith('.tgz'));
  if (tarballs.length !== 1) {
    fail(`expected one .tgz in ${outDir}, found: ${tarballs.join(', ') || '(none)'}`);
  }

  const tarball = join(outDir, tarballs[0]);
  execFileSync('tar', ['-xzf', tarball, '-C', outDir], { stdio: 'inherit' });

  const packed = JSON.parse(readFileSync(join(outDir, 'package', 'package.json'), 'utf8'));
  const coreDep = packed.dependencies?.['@sveda-ai/core'];
  const protocolDep = packed.dependencies?.['@sveda-ai/protocol'];

  if (!isSemverRange(coreDep)) {
    fail(`dependencies["@sveda-ai/core"] must be a semver range, got: ${JSON.stringify(coreDep)}`);
  }
  if (!isSemverRange(protocolDep)) {
    fail(
      `dependencies["@sveda-ai/protocol"] must be a semver range, got: ${JSON.stringify(protocolDep)}`,
    );
  }

  const importPath = packed.exports?.['.']?.import;
  if (importPath !== './dist/index.js') {
    fail(`exports["."].import must be "./dist/index.js", got: ${JSON.stringify(importPath)}`);
  }

  if (!packed.exports?.['./styles.css']) {
    fail('exports["./styles.css"] must exist');
  }

  console.log(`assert-vue-pack: OK (${basename(tarball)})`);
  console.log(
    JSON.stringify(
      {
        name: packed.name,
        version: packed.version,
        dependencies: {
          '@sveda-ai/core': coreDep,
          '@sveda-ai/protocol': protocolDep,
        },
        exports: {
          '.': packed.exports['.'],
          './styles.css': packed.exports['./styles.css'],
        },
      },
      null,
      2,
    ),
  );
} catch (err) {
  fail(err?.stderr || err?.message || String(err));
} finally {
  try {
    rmSync(outDir, { recursive: true, force: true });
  } catch {
    // ignore
  }
}
