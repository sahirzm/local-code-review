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
  test: {
    environment: 'jsdom',
  },
});
