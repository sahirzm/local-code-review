import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  root: '.',
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
  // @pierre/diffs tokenizes in a Web Worker. `?worker&inline` embeds it as a
  // base64 blob so it survives being served from the Rust rust_embed layer
  // without a separate worker-asset URL.
  worker: {
    format: 'es',
  },
  optimizeDeps: {
    // Pre-bundle the worker entry's deps (shiki) so dev-server cold starts
    // don't stall on first tokenization.
    include: ['@pierre/diffs', '@pierre/diffs/react'],
  },
  resolve: {
    alias: {
      '../../../shared/types.js': path.resolve(__dirname, 'src/shared/types.ts'),
    },
  },
  server: {
    proxy: {
      '/api': 'http://127.0.0.1:8989',
    },
  },
});
