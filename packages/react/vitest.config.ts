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
      'lucide-react': fileURLToPath(
        new URL('../../../sveda-sdk-playground/react/node_modules/lucide-react', import.meta.url)
      ),
      clsx: fileURLToPath(new URL('../../../sveda-sdk-playground/react/node_modules/clsx', import.meta.url)),
      'tailwind-merge': fileURLToPath(
        new URL('../../../sveda-sdk-playground/react/node_modules/tailwind-merge', import.meta.url)
      ),
      react: fileURLToPath(new URL('../../../sveda-sdk-playground/react/node_modules/react', import.meta.url)),
      'react-dom': fileURLToPath(
        new URL('../../../sveda-sdk-playground/react/node_modules/react-dom', import.meta.url)
      ),
      'react/jsx-runtime': fileURLToPath(
        new URL('../../../sveda-sdk-playground/react/node_modules/react/jsx-runtime.js', import.meta.url)
      ),
      'react/jsx-dev-runtime': fileURLToPath(
        new URL('../../../sveda-sdk-playground/react/node_modules/react/jsx-dev-runtime.js', import.meta.url)
      ),
      '@testing-library/react': fileURLToPath(
        new URL('../../../sveda-sdk-playground/react/node_modules/@testing-library/react', import.meta.url)
      ),
    },
  },
  test: {
    environment: 'happy-dom',
    include: ['tests/**/*.test.tsx', 'tests/**/*.test.ts'],
  },
});
