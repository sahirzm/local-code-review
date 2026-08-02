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

async function openSettings(screen: ReturnType<typeof render>): Promise<void> {
  await screen.getByLabelText('Settings').click();
  await expect.element(screen.getByLabelText('Select code font')).toBeInTheDocument();
}

function cssVar(name: string): string {
  return document.documentElement.style.getPropertyValue(name).trim();
}

describe('font family controls (browser)', () => {
  let api: MockApiHandle;

  beforeEach(() => {
    localStorage.clear();
    document.documentElement.style.removeProperty('--font-mono');
    document.documentElement.style.removeProperty('--font-sans');
    document.documentElement.style.removeProperty('--diffs-font-family');
    api = mockApi();
  });

  afterEach(() => {
    api.restore();
  });

  it('drives the code font into both --font-mono and pierre --diffs-font-family', async () => {
    const screen = render(<App />);
    await waitForApp(screen);
    await openSettings(screen);

    await screen.getByLabelText('Select code font').selectOptions('JetBrains Mono');

    expect(cssVar('--font-mono')).toContain('JetBrains Mono');
    // Same stack must reach pierre's shadow DOM code font.
    expect(cssVar('--diffs-font-family')).toContain('JetBrains Mono');
    // UI font stays untouched.
    expect(cssVar('--font-sans')).not.toContain('JetBrains Mono');
  });

  it('drives the UI font into --font-sans only', async () => {
    const screen = render(<App />);
    await waitForApp(screen);
    await openSettings(screen);

    await screen.getByLabelText('Select UI font').selectOptions('Roboto');

    expect(cssVar('--font-sans')).toContain('Roboto');
    expect(cssVar('--diffs-font-family')).not.toContain('Roboto');
  });

  it('persists both font choices to localStorage', async () => {
    const screen = render(<App />);
    await waitForApp(screen);
    await openSettings(screen);

    await screen.getByLabelText('Select code font').selectOptions('Fira Code');
    await screen.getByLabelText('Select UI font').selectOptions('Helvetica');

    const prefs = JSON.parse(localStorage.getItem(PREFS_KEY) ?? '{}') as { codeFont?: string; uiFont?: string };
    expect(prefs.codeFont).toBe('fira-code');
    expect(prefs.uiFont).toBe('helvetica');
  });

  it('restores the persisted code font on load', async () => {
    localStorage.setItem(PREFS_KEY, JSON.stringify({ codeFont: 'ibm-plex-mono' }));
    const screen = render(<App />);
    await waitForApp(screen);

    expect(cssVar('--diffs-font-family')).toContain('IBM Plex Mono');
    await openSettings(screen);
    await expect.element(screen.getByLabelText('Select code font')).toHaveValue('ibm-plex-mono');
  });
});
