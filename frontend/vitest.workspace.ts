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
      setupFiles: ['./src/test/setup-unit.ts'],
      include: ['src/**/*.{test,spec}.{ts,tsx}'],
      exclude: ['src/**/*.browser.test.tsx', 'node_modules/**', 'dist/**'],
    },
  },
  {
    extends: './vite.config.ts',
    test: {
      name: 'browser',
      include: ['src/**/*.browser.test.tsx'],
      // Run browser test files one at a time. They share a single headless
      // Chromium, and the @pierre/diffs worker pool that the diff surface spins
      // up makes concurrent files contend for it — starving locator actions
      // enough to trip their default 1s timeout intermittently.
      fileParallelism: false,
      browser: {
        enabled: true,
        provider: 'playwright',
        headless: true,
        name: 'chromium',
      },
    },
  },
]);
