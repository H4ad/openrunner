import { resolve } from 'path';
import { defineConfig, externalizeDepsPlugin } from 'electron-vite';
import vue from '@vitejs/plugin-vue';
import tailwindcss from '@tailwindcss/vite';

export default defineConfig({
  main: {
    plugins: [
      externalizeDepsPlugin({
        // Bundle CLI dependencies since the CLI runs standalone without node_modules
        // better-sqlite3 is native and must stay external (copied separately)
        exclude: ['commander', '@inquirer/prompts', 'js-yaml'],
      }),
    ],
    build: {
      rollupOptions: {
        input: {
          index: resolve(__dirname, 'src-electron/main/index.ts'),
          cli: resolve(__dirname, 'src-electron/cli/standalone.ts'),
        },
      },
    },
    resolve: {
      alias: {
        '@': resolve(__dirname, './src'),
      },
    },
  },
  preload: {
    plugins: [externalizeDepsPlugin()],
    build: {
      rollupOptions: {
        input: {
          index: resolve(__dirname, 'src-electron/preload/index.ts'),
        },
      },
    },
  },
  renderer: {
    root: '.',
    build: {
      rollupOptions: {
        input: {
          index: resolve(__dirname, 'index.html'),
        },
      },
    },
    plugins: [vue(), tailwindcss()],
    resolve: {
      alias: {
        '@': resolve(__dirname, './src'),
      },
    },
  },
});
