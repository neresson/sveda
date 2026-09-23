import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  esbuild: {
    jsx: 'automatic',
  },
  resolve: {
    alias: {
      '@sveda-ai/chat': fileURLToPath(new URL('../chat/src', import.meta.url)),
      '@sveda-ai/core': fileURLToPath(new URL('../core/src', import.meta.url)),
      '@sveda-ai/protocol': fileURLToPath(new URL('../protocol/src', import.meta.url)),
    },
    dedupe: ['react', 'react-dom'],
  },
  test: {
    environment: 'happy-dom',
    include: ['tests/**/*.test.tsx', 'tests/**/*.test.ts'],
  },
});
