/// <reference types="@vitest/browser/context" />
/// <reference types="@vitest/browser/matchers" />
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-react';
import { App } from './App.js';
import { mockApi, type MockApiHandle } from './test/mock-api.js';

const PREFS_KEY = 'local-review:preferences';

async function waitForApp(screen: ReturnType<typeof render>): Promise<void> {
  await expect.element(screen.getByText('demo-repo')).toBeInTheDocument();
}

function bodyBg(): string {
  return getComputedStyle(document.body).backgroundColor;
}

describe('theme selector (browser)', () => {
  let api: MockApiHandle;

  beforeEach(() => {
    localStorage.clear();
    document.documentElement.removeAttribute('data-theme');
    api = mockApi();
  });

  afterEach(() => {
    api.restore();
  });

  it('defaults to the dark theme', async () => {
    const screen = render(<App />);
    await waitForApp(screen);
    expect(document.documentElement.getAttribute('data-theme')).toBe('default-dark');
    // #1e1e1e
    expect(bodyBg()).toBe('rgb(30, 30, 30)');
  });

  it('switches to Catppuccin Mocha and updates the background', async () => {
    const screen = render(<App />);
    await waitForApp(screen);

    const select = screen.getByLabelText('Select color theme');
    await select.selectOptions('Catppuccin Mocha');

    expect(document.documentElement.getAttribute('data-theme')).toBe('catppuccin-mocha');
    // Mocha base #1e1e2e
    expect(bodyBg()).toBe('rgb(30, 30, 46)');
  });

  it('switches to a light theme', async () => {
    const screen = render(<App />);
    await waitForApp(screen);

    const select = screen.getByLabelText('Select color theme');
    await select.selectOptions('Default Light');

    expect(document.documentElement.getAttribute('data-theme')).toBe('default-light');
    // #ffffff
    expect(bodyBg()).toBe('rgb(255, 255, 255)');
  });

  it('persists the chosen theme to localStorage', async () => {
    const screen = render(<App />);
    await waitForApp(screen);

    const select = screen.getByLabelText('Select color theme');
    await select.selectOptions('Catppuccin Latte');

    const prefs = JSON.parse(localStorage.getItem(PREFS_KEY) ?? '{}') as { theme?: string };
    expect(prefs.theme).toBe('catppuccin-latte');
  });

  it('migrates the legacy "light" preference to default-light', async () => {
    localStorage.setItem(PREFS_KEY, JSON.stringify({ theme: 'light' }));
    const screen = render(<App />);
    await waitForApp(screen);
    expect(document.documentElement.getAttribute('data-theme')).toBe('default-light');
  });
});
