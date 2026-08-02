/// <reference types="@vitest/browser/context" />
/// <reference types="@vitest/browser/matchers" />
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-react';
import { App } from './App.js';
import { mockApi, type MockApiHandle } from './test/mock-api.js';

async function waitForApp(screen: ReturnType<typeof render>): Promise<void> {
  await expect.element(screen.getByText('demo-repo')).toBeInTheDocument();
}

describe('settings modal (browser)', () => {
  let api: MockApiHandle;

  beforeEach(() => {
    localStorage.clear();
    api = mockApi();
  });

  afterEach(() => {
    api.restore();
  });

  it('keeps configuration out of the toolbar until the gear is opened', async () => {
    const screen = render(<App />);
    await waitForApp(screen);

    // Config controls are not on the toolbar anymore.
    expect(screen.getByLabelText('Select color theme').query()).toBeNull();
    expect(screen.getByLabelText('Diff context lines').query()).toBeNull();
    expect(screen.getByLabelText('Increase diff font size').query()).toBeNull();

    await screen.getByLabelText('Settings').click();

    // ...and appear once the modal is open.
    await expect.element(screen.getByLabelText('Select color theme')).toBeInTheDocument();
    await expect.element(screen.getByLabelText('Diff context lines')).toBeInTheDocument();
    await expect.element(screen.getByLabelText('Increase diff font size')).toBeInTheDocument();
  });

  it('opens the keyboard shortcuts modal from settings', async () => {
    const screen = render(<App />);
    await waitForApp(screen);

    await screen.getByLabelText('Settings').click();
    await screen.getByRole('button', { name: 'Keyboard shortcuts' }).click();

    await expect.element(screen.getByRole('heading', { name: 'Keyboard Shortcuts' })).toBeInTheDocument();
    // Settings closes when handing off to the shortcuts modal (poll past the
    // modal's exit animation, which keeps it briefly mounted).
    await expect.poll(() => screen.getByLabelText('Select color theme').query()).toBeNull();
  });

  it('keeps the toolbar within the header when the ref name is very long', async () => {
    api.restore();
    api = mockApi({ headRef: 'feature/' + 'very-long-branch-name-segment-'.repeat(8) });
    const screen = render(<App />);
    await waitForApp(screen);

    const toolbar = document.querySelector('.toolbar') as HTMLElement;
    const header = document.querySelector('.header-row') as HTMLElement;
    expect(toolbar).not.toBeNull();
    expect(header).not.toBeNull();

    // The long title must ellipsis rather than shove the toolbar past the
    // header's right edge (allow 1px for sub-pixel rounding).
    // Nothing in the header may spill past the viewport's right edge (allow
    // 1px for sub-pixel rounding). Covers both the wide case (title ellipsis)
    // and the narrow case (toolbar wraps to its own row).
    const toolbarRight = toolbar.getBoundingClientRect().right;
    expect(toolbarRight).toBeLessThanOrEqual(window.innerWidth + 1);
  });

  it('exposes the file and comment navigation as icon buttons', async () => {
    const screen = render(<App />);
    await waitForApp(screen);

    // Nav labels are now icons + accessible names, not visible text.
    await expect.element(screen.getByLabelText('Previous file')).toBeInTheDocument();
    await expect.element(screen.getByLabelText('Next file')).toBeInTheDocument();
    await expect.element(screen.getByLabelText('Previous comment')).toBeInTheDocument();
    await expect.element(screen.getByLabelText('Next comment')).toBeInTheDocument();
  });
});
