import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vite';
import tailwindcss from '@tailwindcss/vite';
import vue from '@vitejs/plugin-vue';

const root = dirname(fileURLToPath(import.meta.url));
const svedaPackages = resolve(root, '../../packages');

export default defineConfig({
    base: '/build/',
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
    resolve: {
        alias: {
            '@sveda-ai/protocol': resolve(svedaPackages, 'protocol/src'),
            '@sveda-ai/core': resolve(svedaPackages, 'core/src'),
            '@sveda-ai/vue': resolve(svedaPackages, 'vue/src'),
        },
        dedupe: ['vue'],
    },
    build: {
        outDir: resolve(root, 'public/build'),
        emptyOutDir: true,
        manifest: true,
        rollupOptions: {
            input: 'resources/js/admin/app.js',
        },
    },
});
