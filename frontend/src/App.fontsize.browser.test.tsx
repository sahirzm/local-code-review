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
  return document.documentElement.style.getPropertyValue('--diff-font-size').trim();
}

async function waitForApp(screen: ReturnType<typeof render>): Promise<void> {
  await expect.element(screen.getByText('demo-repo')).toBeInTheDocument();
}

describe('diff font size control (browser)', () => {
  let api: MockApiHandle;

  beforeEach(() => {
    localStorage.clear();
    document.documentElement.style.removeProperty('--diff-font-size');
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

    await screen.getByLabelText('Increase diff font size').click();
    expect(diffFontSizeVar()).toBe(`${DEFAULT_FONT_SIZE + 1}px`);

    await screen.getByLabelText('Decrease diff font size').click();
    await screen.getByLabelText('Decrease diff font size').click();
    expect(diffFontSizeVar()).toBe(`${DEFAULT_FONT_SIZE - 1}px`);
  });

  it('clamps at the maximum and disables the increase button', async () => {
    const screen = render(<App />);
    await waitForApp(screen);

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

    const decrease = screen.getByLabelText('Decrease diff font size');
    for (let i = 0; i < DEFAULT_FONT_SIZE - MIN_FONT_SIZE + 3; i++) {
      if (decrease.query()?.hasAttribute('disabled')) break;
      await decrease.click();
    }
    expect(diffFontSizeVar()).toBe(`${MIN_FONT_SIZE}px`);
    await expect.element(decrease).toBeDisabled();
  });

  it('persists the font size to localStorage', async () => {
    const screen = render(<App />);
    await waitForApp(screen);

    await screen.getByLabelText('Increase diff font size').click();

    const prefs = JSON.parse(localStorage.getItem(PREFS_KEY) ?? '{}') as { fontSize?: number };
    expect(prefs.fontSize).toBe(DEFAULT_FONT_SIZE + 1);
  });
});
