import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vitest/config';
import solid from 'vite-plugin-solid';

export default defineConfig({
  plugins: [solid()],
  resolve: {
    alias: {
      '@sveda-ai/chat': fileURLToPath(new URL('../chat/src', import.meta.url)),
      '@sveda-ai/core': fileURLToPath(new URL('../core/src', import.meta.url)),
      '@sveda-ai/protocol': fileURLToPath(new URL('../protocol/src', import.meta.url)),
    },
    conditions: ['development', 'browser'],
  },
  test: {
    environment: 'happy-dom',
    include: ['tests/**/*.test.tsx', 'tests/**/*.test.ts'],
  },
});
