/// <reference types="@vitest/browser/context" />
/// <reference types="@vitest/browser/matchers" />
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { page } from '@vitest/browser/context';
import { render } from 'vitest-browser-react';
import { App } from './App.js';
import { mockApi, type MockApiHandle } from './test/mock-api.js';

/**
 * Overall comments used to occupy a strip above the diff on every file, whether
 * or not there were any. They now live behind a sidebar button and a modal.
 */
describe('overall comments (browser)', () => {
  let api: MockApiHandle;

  beforeEach(async () => {
    localStorage.clear();
    api = mockApi();
    // The test iframe defaults to a tall mobile-ish box that extends past the
    // real browser window, which puts anything pinned to the bottom of the
    // layout out of reach of a real click. Shrink it to something clickable.
    await page.viewport(1100, 680);
  });

  afterEach(() => {
    api.restore();
  });

  it('costs the diff column no vertical space', async () => {
    const screen = render(<App />);
    await expect.element(screen.getByText('demo-repo')).toBeInTheDocument();

    expect(document.querySelector('.overall-comments')).toBeNull();

    // The diff scroller is the only flexible child left under the header.
    const main = document.querySelector('.app-main') as HTMLElement;
    const scroll = document.querySelector('.diff-view-scroll') as HTMLElement;
    const header = document.querySelector('.app-main .header') as HTMLElement;
    expect(main).not.toBeNull();
    expect(scroll.getBoundingClientRect().height).toBeGreaterThan(
      main.getBoundingClientRect().height - header.getBoundingClientRect().height - 2,
    );
  });

  it('opens the composer from the sidebar and records the comment', async () => {
    const screen = render(<App />);
    await expect.element(screen.getByText('demo-repo')).toBeInTheDocument();

    await screen.getByRole('button', { name: /Overall comment/ }).click();
    const form = screen.getByRole('form', { name: 'Add comment' });
    await expect.element(form).toBeInTheDocument();

    await screen.getByLabelText('Comment text').fill('ship it');
    await screen.getByRole('button', { name: 'Submit' }).click();

    // Modal closes (after its exit animation), and the sidebar button carries
    // the running count.
    await expect.element(screen.getByLabelText('1 so far')).toBeInTheDocument();
    await expect.element(screen.getByRole('form', { name: 'Add comment' })).not.toBeInTheDocument();
  });

  it('opens the composer with the c shortcut', async () => {
    const screen = render(<App />);
    await expect.element(screen.getByText('demo-repo')).toBeInTheDocument();

    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'c', bubbles: true }));

    await expect.element(screen.getByRole('form', { name: 'Add comment' })).toBeInTheDocument();
  });
});
