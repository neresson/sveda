import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';
import tailwindcss from '@tailwindcss/vite';

const root = dirname(fileURLToPath(import.meta.url));
const svedaPackages = resolve(root, '../../packages');

export default defineConfig({
    publicDir: false,
    plugins: [
        vue({
            template: {
                transformAssetUrls: {
                    base: null,
                    includeAbsolute: false,
                },
            },
        }),
        tailwindcss(),
    ],
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
        outDir: resolve(root, 'public/build/sveda'),
        emptyOutDir: false,
        rollupOptions: {
            input: resolve(root, 'resources/js/embed/app.js'),
            output: {
                entryFileNames: 'embed.js',
                assetFileNames: 'embed[extname]',
            },
        },
    },
});
