/// <reference types="@vitest/browser/context" />
/// <reference types="@vitest/browser/matchers" />
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-react';
import { App } from './App.js';
import { PREFS_KEY } from './utils/preferences.js';
import { mockApi, type MockApiHandle } from './test/mock-api.js';

/**
 * Preferences that lived under their own `local-review:*` keys were deleted on
 * every mount by `cleanExpiredSessions()`, which treated the whole namespace as
 * expiring review sessions. These cover the consolidated blob.
 */
describe('preference persistence (browser)', () => {
  let api: MockApiHandle;

  beforeEach(() => {
    localStorage.clear();
    api = mockApi();
  });

  afterEach(() => {
    api.restore();
  });

  it('keeps the diff context setting across a remount', async () => {
    const first = render(<App />);
    await expect.element(first.getByText('demo-repo')).toBeInTheDocument();

    await first.getByLabelText('Settings').click();
    await first.getByLabelText('Diff context lines').selectOptions('Full context');

    await expect
      .poll(() => JSON.parse(localStorage.getItem(PREFS_KEY) ?? '{}').contextLevel)
      .toBe('full');

    first.unmount();

    const second = render(<App />);
    await expect.element(second.getByText('demo-repo')).toBeInTheDocument();
    await second.getByLabelText('Settings').click();
    await expect.element(second.getByLabelText('Diff context lines')).toHaveValue('full');
  });

  it('does not strand preferences under keys the session sweep deletes', async () => {
    const screen = render(<App />);
    await expect.element(screen.getByText('demo-repo')).toBeInTheDocument();

    await screen.getByLabelText('Settings').click();
    await screen.getByLabelText('Diff context lines').selectOptions('20 lines');

    await expect
      .poll(() => JSON.parse(localStorage.getItem(PREFS_KEY) ?? '{}').contextLevel)
      .toBe(20);
    expect(localStorage.getItem('local-review:diff-context')).toBeNull();
    expect(localStorage.getItem('local-review:sidebar-width')).toBeNull();
  });
});
