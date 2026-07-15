/// <reference types="@vitest/browser/context" />
/// <reference types="@vitest/browser/matchers" />
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-react';
import { App } from './App.js';
import { mockApi, type MockApiHandle } from './test/mock-api.js';

describe('App (browser smoke)', () => {
  let api: MockApiHandle;

  beforeEach(() => {
    localStorage.clear();
    api = mockApi();
  });

  afterEach(() => {
    api.restore();
  });

  it('loads the review UI against the mocked API', async () => {
    const screen = render(<App />);

    // Header renders the repo name once metadata resolves.
    await expect.element(screen.getByText('demo-repo')).toBeInTheDocument();

    // The file tree lists a mocked file (exact match avoids the diff's "src/index.ts").
    await expect.element(screen.getByText('index.ts', { exact: true })).toBeInTheDocument();
  });
});
