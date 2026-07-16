import { afterEach } from 'vitest';
import { cleanup } from '@testing-library/react';
import '@testing-library/jest-dom/vitest';

// Unmount React trees between tests so `screen` queries don't accumulate
// across the shared jsdom document.
afterEach(() => {
  cleanup();
});
