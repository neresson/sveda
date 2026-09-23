import { fileURLToPath } from 'node:url';
import vue from '@vitejs/plugin-vue';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      '@sveda-ai/chat': fileURLToPath(new URL('../chat/src', import.meta.url)),
      '@sveda-ai/core': fileURLToPath(new URL('../core/src', import.meta.url)),
      '@sveda-ai/protocol': fileURLToPath(new URL('../protocol/src', import.meta.url)),
    },
  },
  test: {
    environment: 'happy-dom',
    include: ['tests/**/*.test.ts'],
    coverage: {
      provider: 'v8',
      thresholds: { lines: 60 },
    },
  },
});
