/// <reference types="@vitest/browser/context" />
/// <reference types="@vitest/browser/matchers" />
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-react';
import { App } from './App.js';
import { mockApi, type MockApiHandle } from './test/mock-api.js';

const PREFS_KEY = 'local-review:preferences';
const DEFAULT_FONT_SIZE = 13;
const MIN_FONT_SIZE = 10;
const MAX_FONT_SIZE = 20;

function diffFontSizeVar(): string {
  return document.documentElement.style.getPropertyValue('--diffs-font-size').trim();
}

async function waitForApp(screen: ReturnType<typeof render>): Promise<void> {
  await expect.element(screen.getByText('demo-repo')).toBeInTheDocument();
}

// The font size control now lives behind the toolbar gear.
async function openSettings(screen: ReturnType<typeof render>): Promise<void> {
  await screen.getByLabelText('Settings').click();
  await expect.element(screen.getByLabelText('Increase diff font size')).toBeInTheDocument();
}

describe('diff font size control (browser)', () => {
  let api: MockApiHandle;

  beforeEach(() => {
    localStorage.clear();
    document.documentElement.style.removeProperty('--diffs-font-size');
    document.documentElement.style.removeProperty('--diffs-line-height');
    api = mockApi();
  });

  afterEach(() => {
    api.restore();
  });

  it('applies the default font size on load', async () => {
    const screen = render(<App />);
    await waitForApp(screen);
    expect(diffFontSizeVar()).toBe(`${DEFAULT_FONT_SIZE}px`);
  });

  it('increases and decreases the font size', async () => {
    const screen = render(<App />);
    await waitForApp(screen);
    await openSettings(screen);

    await screen.getByLabelText('Increase diff font size').click();
    expect(diffFontSizeVar()).toBe(`${DEFAULT_FONT_SIZE + 1}px`);

    await screen.getByLabelText('Decrease diff font size').click();
    await screen.getByLabelText('Decrease diff font size').click();
    expect(diffFontSizeVar()).toBe(`${DEFAULT_FONT_SIZE - 1}px`);
  });

  it('clamps at the maximum and disables the increase button', async () => {
    const screen = render(<App />);
    await waitForApp(screen);
    await openSettings(screen);

    const increase = screen.getByLabelText('Increase diff font size');
    for (let i = 0; i < MAX_FONT_SIZE - DEFAULT_FONT_SIZE + 3; i++) {
      if (increase.query()?.hasAttribute('disabled')) break;
      await increase.click();
    }
    expect(diffFontSizeVar()).toBe(`${MAX_FONT_SIZE}px`);
    await expect.element(increase).toBeDisabled();
  });

  it('clamps at the minimum and disables the decrease button', async () => {
    const screen = render(<App />);
    await waitForApp(screen);
    await openSettings(screen);

    const decrease = screen.getByLabelText('Decrease diff font size');
    for (let i = 0; i < DEFAULT_FONT_SIZE - MIN_FONT_SIZE + 3; i++) {
      if (decrease.query()?.hasAttribute('disabled')) break;
      await decrease.click();
    }
    expect(diffFontSizeVar()).toBe(`${MIN_FONT_SIZE}px`);
    await expect.element(decrease).toBeDisabled();
  });

  it('drives the rendered diff font size inside the pierre shadow DOM', async () => {
    const screen = render(<App />);
    await waitForApp(screen);

    // Pierre reads `--diffs-font-size` on its shadow host (`:host { font-size:
    // var(--diffs-font-size,13px) }`); the custom property inherits through the
    // shadow boundary from documentElement. Find the deepest host that resolves
    // to the current font size, then assert it tracks the control.
    const shadowHostFontSize = (): number | null => {
      let found: number | null = null;
      const walk = (root: ParentNode): void => {
        for (const el of root.querySelectorAll('*')) {
          if (el.shadowRoot) {
            found = parseFloat(getComputedStyle(el).fontSize);
            walk(el.shadowRoot);
          }
        }
      };
      walk(document);
      return found;
    };

    await expect
      .poll(() => shadowHostFontSize(), { timeout: 15_000, interval: 250 })
      .toBe(DEFAULT_FONT_SIZE);

    await openSettings(screen);
    await screen.getByLabelText('Increase diff font size').click();
    await expect
      .poll(() => shadowHostFontSize(), { timeout: 5_000, interval: 100 })
      .toBe(DEFAULT_FONT_SIZE + 1);
  });

  it('persists the font size to localStorage', async () => {
    const screen = render(<App />);
    await waitForApp(screen);
    await openSettings(screen);

    await screen.getByLabelText('Increase diff font size').click();

    const prefs = JSON.parse(localStorage.getItem(PREFS_KEY) ?? '{}') as { fontSize?: number };
    expect(prefs.fontSize).toBe(DEFAULT_FONT_SIZE + 1);
  });
});
