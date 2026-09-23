import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vitest/config';

const packageRoot = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  resolve: {
    alias: {
      '@sveda-ai/core': path.resolve(packageRoot, '../core/src'),
      '@sveda-ai/protocol': path.resolve(packageRoot, '../protocol/src'),
    },
  },
  test: {
    environment: 'happy-dom',
    include: ['tests/**/*.test.ts'],
    coverage: {
      provider: 'v8',
      include: [
        'src/appearance.ts',
        'src/lib/chatHistoryMerge.ts',
        'src/lib/chatMessageVisibility.ts',
        'src/lib/chatResize.ts',
        'src/lib/chatUiStorage.ts',
        'src/lib/markdown.ts',
        'src/lib/toolConfirmation.ts',
      ],
      thresholds: { lines: 60 },
    },
  },
});
