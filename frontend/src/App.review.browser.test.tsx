/// <reference types="@vitest/browser/context" />
/// <reference types="@vitest/browser/matchers" />
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-react';
import { App } from './App.js';
import { mockApi, type MockApiHandle } from './test/mock-api.js';

/**
 * The review action used to live only in the top toolbar — a full viewport away
 * from where a reader actually finishes a file. These cover the replacement:
 * an end-of-diff action bar that also advances to the next unreviewed file.
 */
describe('end-of-file review action (browser)', () => {
  let api: MockApiHandle;

  beforeEach(() => {
    localStorage.clear();
    api = mockApi();
  });

  afterEach(() => {
    api.restore();
  });

  it('marks the file reviewed and advances to the next unreviewed file', async () => {
    const screen = render(<App />);
    await expect.element(screen.getByText('demo-repo')).toBeInTheDocument();
    await expect.element(screen.getByText('File 1 of 4')).toBeInTheDocument();

    await screen.getByRole('button', { name: /Mark reviewed & next/ }).click();

    // Advanced to file 2, and the bar now offers the action for that file.
    await expect.element(screen.getByText('File 2 of 4')).toBeInTheDocument();
    await expect
      .element(screen.getByRole('button', { name: /Mark reviewed & next/ }))
      .toBeInTheDocument();
  });

  it('offers an undo instead of a second action once a file is reviewed', async () => {
    const screen = render(<App />);
    await expect.element(screen.getByText('demo-repo')).toBeInTheDocument();

    // `r` marks the current file without advancing, so the bar should flip to
    // the undo affordance for the file still on screen.
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'r', bubbles: true }));

    await expect.element(screen.getByText('File 1 of 4')).toBeInTheDocument();
    await expect.element(screen.getByRole('button', { name: /Reviewed — undo/ })).toBeInTheDocument();
  });

  it('keeps the file header pinned while the diff scrolls', async () => {
    const screen = render(<App />);
    await expect.element(screen.getByText('demo-repo')).toBeInTheDocument();

    const header = document.querySelector('.file-header-row');
    expect(header).not.toBeNull();
    expect(getComputedStyle(header as Element).position).toBe('sticky');

    // `overflow: hidden` on the wrapper would make it the sticky containing
    // block, silently pinning the header to a box that never scrolls.
    const wrapper = document.querySelector('.file-diff');
    expect(getComputedStyle(wrapper as Element).overflow).not.toBe('hidden');
  });
});
