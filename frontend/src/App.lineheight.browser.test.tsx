/// <reference types="@vitest/browser/context" />
/// <reference types="@vitest/browser/matchers" />
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-react';
import { App } from './App.js';
import { mockApi, type MockApiHandle } from './test/mock-api.js';

const DEFAULT_FONT_SIZE = 13;
// 20/13 rounded to one decimal, matching App's DEFAULT_LINE_HEIGHT.
const DEFAULT_LINE_HEIGHT = Math.round((20 / 13) * 10) / 10;
const MIN_LINE_HEIGHT = 1.2;
const MAX_LINE_HEIGHT = 2.4;

function lineHeightVar(): string {
  return document.documentElement.style.getPropertyValue('--diffs-line-height').trim();
}

async function waitForApp(screen: ReturnType<typeof render>): Promise<void> {
  await expect.element(screen.getByText('demo-repo')).toBeInTheDocument();
}

async function openSettings(screen: ReturnType<typeof render>): Promise<void> {
  await screen.getByLabelText('Settings').click();
  await expect.element(screen.getByLabelText('Increase diff line height')).toBeInTheDocument();
}

describe('diff line-height control (browser)', () => {
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

  it('applies the default line-height on load', async () => {
    const screen = render(<App />);
    await waitForApp(screen);
    // Rendered row height is round(fontSize * lineHeight)px.
    expect(lineHeightVar()).toBe(`${Math.round(DEFAULT_FONT_SIZE * DEFAULT_LINE_HEIGHT)}px`);
  });

  it('increases and decreases the line-height', async () => {
    const screen = render(<App />);
    await waitForApp(screen);
    await openSettings(screen);

    await screen.getByLabelText('Increase diff line height').click();
    const increased = Math.round(DEFAULT_LINE_HEIGHT * 10 + 1) / 10;
    expect(lineHeightVar()).toBe(`${Math.round(DEFAULT_FONT_SIZE * increased)}px`);

    await screen.getByLabelText('Decrease diff line height').click();
    await screen.getByLabelText('Decrease diff line height').click();
    const decreased = Math.round(DEFAULT_LINE_HEIGHT * 10 - 1) / 10;
    expect(lineHeightVar()).toBe(`${Math.round(DEFAULT_FONT_SIZE * decreased)}px`);
  });

  it('clamps at the maximum and disables the increase button', async () => {
    localStorage.setItem(
      'local-review:preferences',
      JSON.stringify({ lineHeight: MAX_LINE_HEIGHT }),
    );
    const screen = render(<App />);
    await waitForApp(screen);
    await openSettings(screen);
    await expect.element(screen.getByLabelText('Increase diff line height')).toBeDisabled();
  });

  it('clamps at the minimum and disables the decrease button', async () => {
    localStorage.setItem(
      'local-review:preferences',
      JSON.stringify({ lineHeight: MIN_LINE_HEIGHT }),
    );
    const screen = render(<App />);
    await waitForApp(screen);
    await openSettings(screen);
    await expect.element(screen.getByLabelText('Decrease diff line height')).toBeDisabled();
  });
});
