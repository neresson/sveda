import vue from '@vitejs/plugin-vue';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: [
      {
        find: /^@veda-ai\/vue\/styles\.css(\?inline)?$/,
        replacement: fileURLToPath(new URL('./tests/stubs/styles.css', import.meta.url)),
      },
    ],
  },
  test: {
    environment: 'happy-dom',
    include: ['tests/**/*.test.ts'],
  },
});
