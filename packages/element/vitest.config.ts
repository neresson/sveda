import vue from '@vitejs/plugin-vue';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: [
      {
        find: /^@sveda-ai\/vue\/styles\.css(\?inline)?$/,
        replacement: fileURLToPath(new URL('./tests/stubs/styles.css', import.meta.url)),
      },
      {
        find: '@sveda-ai/vue',
        replacement: fileURLToPath(new URL('../vue/src', import.meta.url)),
      },
      {
        find: '@sveda-ai/core',
        replacement: fileURLToPath(new URL('../core/src', import.meta.url)),
      },
      {
        find: '@sveda-ai/protocol',
        replacement: fileURLToPath(new URL('../protocol/src', import.meta.url)),
      },
    ],
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
