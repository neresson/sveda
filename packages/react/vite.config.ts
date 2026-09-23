import { fileURLToPath } from 'node:url';
import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [react()],
  build: {
    lib: {
      entry: {
        index: fileURLToPath(new URL('./src/index.ts', import.meta.url)),
      },
      formats: ['es'],
      fileName: (_format, entryName) => `${entryName}.js`,
    },
    rollupOptions: {
      external: (id) =>
        id === 'react' ||
        id === 'react-dom' ||
        id === 'react/jsx-runtime' ||
        id.startsWith('react/') ||
        id.startsWith('react-dom/') ||
        id.startsWith('@sveda-ai/') ||
        (!id.startsWith('.') && !id.startsWith('/') && !id.endsWith('.css') && !id.includes('\0')),
    },
  },
});
