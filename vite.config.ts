/// <reference types="vitest/config" />
import { defineConfig } from "vite";
import { resolve } from "path";
import vue from "@vitejs/plugin-vue";
import AutoImport from 'unplugin-auto-import/vite';
import Components from 'unplugin-vue-components/vite';
import { ElementPlusResolver } from 'unplugin-vue-components/resolvers';

const isTest = process.env.VITEST === 'true';

// https://vitejs.dev/config/
export default defineConfig(async () => ({
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
    },
  },
  plugins: [
    vue(),
    ...(isTest ? [] : [
      AutoImport({
        resolvers: [ElementPlusResolver({ importStyle: false })],
      }),
      Components({
        resolvers: [ElementPlusResolver({ importStyle: false })],
      }),
    ]),
  ],

  // Vite options for Tauri
  clearScreen: false,
  build: {
    chunkSizeWarningLimit: 1200,
  },
  server: {
    port: 5176,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },

  // Vitest configuration
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/__tests__/setup.ts'],
    css: true,
  },
}));
