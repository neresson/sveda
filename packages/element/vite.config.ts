import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import vue from '@vitejs/plugin-vue';
import { defineConfig } from 'vite';

const root = dirname(fileURLToPath(import.meta.url));
const svedaPackages = resolve(root, '..');

const emitStandaloneCss = () => ({
  name: 'sveda-standalone-css',
  generateBundle(_options, bundle) {
    for (const chunk of Object.values(bundle)) {
      if (chunk.type !== 'chunk') {
        continue;
      }

      chunk.code = chunk.code.replace(/^import\s+["'][^"']+\.css["'];?\r?\n/gm, '');
    }
  },
});

export default defineConfig({
  plugins: [vue(), emitStandaloneCss()],
  define: {
    'process.env.NODE_ENV': JSON.stringify('production'),
  },
  resolve: {
    alias: {
      '@sveda-ai/protocol': resolve(svedaPackages, 'protocol/src'),
      '@sveda-ai/core': resolve(svedaPackages, 'core/src'),
      '@sveda-ai/vue': resolve(svedaPackages, 'vue/src'),
    },
    dedupe: ['vue'],
  },
  build: {
    lib: {
      entry: resolve(root, 'src/index.ts'),
      name: 'SvedaChatElement',
      formats: ['es'],
      fileName: () => 'sveda-chat.js',
    },
    cssCodeSplit: false,
    outDir: resolve(root, '../../apps/runtime/public/build/sveda'),
    emptyOutDir: false,
    rollupOptions: {
      output: {
        assetFileNames: 'sveda-chat[extname]',
      },
    },
  },
});
