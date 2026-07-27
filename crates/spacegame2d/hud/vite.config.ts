import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';
export default defineConfig({ plugins: [svelte(), tailwindcss()], resolve: { conditions: ['browser'] }, build: { sourcemap: false, rollupOptions: { output: { entryFileNames: 'assets/hud.js', assetFileNames: 'assets/hud.[ext]' } } }, test: { environment: 'jsdom' } });
