import { defineWorkspace } from 'vitest/config';

// Two projects share one `npm test` run:
//   unit    — existing jsdom component/logic tests
//   browser — real headless Chromium (Playwright provider) for *.browser.test.tsx
export default defineWorkspace([
  {
    extends: './vite.config.ts',
    test: {
      name: 'unit',
      environment: 'jsdom',
      include: ['src/**/*.{test,spec}.{ts,tsx}'],
      exclude: ['src/**/*.browser.test.tsx', 'node_modules/**', 'dist/**'],
    },
  },
  {
    extends: './vite.config.ts',
    test: {
      name: 'browser',
      include: ['src/**/*.browser.test.tsx'],
      browser: {
        enabled: true,
        provider: 'playwright',
        headless: true,
        name: 'chromium',
      },
    },
  },
]);
