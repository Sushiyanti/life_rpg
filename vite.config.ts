import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';

// Tauri drives the frontend through a fixed dev URL, so the port must be
// pinned and must fail loudly instead of silently sliding to another port when
// it is already taken.
const DEV_PORT = 1420;

export default defineConfig({
  plugins: [react()],

  // Relative asset paths: Tauri serves the built bundle from an app-local
  // origin, so absolute `/assets/...` URLs would break in the packaged app.
  base: './',

  clearScreen: false,
  server: {
    port: DEV_PORT,
    strictPort: true,
    host: '127.0.0.1',
    watch: {
      // Never watch Rust build output; it would trigger endless reloads.
      ignored: ['**/src-tauri/**', '**/target/**'],
    },
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    target: 'es2022',
    sourcemap: true,
  },
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/test/setup.ts'],
    include: ['src/**/*.test.{ts,tsx}'],
    css: false,
  },
});
