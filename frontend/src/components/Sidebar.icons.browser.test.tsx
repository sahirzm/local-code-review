/// <reference types="@vitest/browser/context" />
/// <reference types="@vitest/browser/matchers" />
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-react';
import { App } from '../App.js';
import { mockApi, type MockApiHandle } from '../test/mock-api.js';

describe('file-type icons in the tree (browser)', () => {
  let api: MockApiHandle;

  beforeEach(() => {
    localStorage.clear();
    api = mockApi();
  });

  afterEach(() => {
    api.restore();
  });

  it('renders an svg icon on every tree file row', async () => {
    const screen = render(<App />);
    await expect.element(screen.getByText('index.ts', { exact: true })).toBeInTheDocument();

    const fileButtons = Array.from(document.querySelectorAll('.tree-file-btn'));
    expect(fileButtons.length).toBeGreaterThan(0);
    for (const btn of fileButtons) {
      expect(btn.querySelector('svg.tree-file-icon')).not.toBeNull();
    }
  });

  it('uses different icons for different extensions', async () => {
    const screen = render(<App />);
    await expect.element(screen.getByText('index.ts', { exact: true })).toBeInTheDocument();

    function iconClassFor(fileName: string): string {
      const nameEl = Array.from(document.querySelectorAll('.tree-file-name')).find(
        (el) => el.textContent === fileName,
      );
      const svg = nameEl?.parentElement?.querySelector('svg.tree-file-icon');
      return svg?.getAttribute('class') ?? '';
    }

    // .ts (code), .py (code), .json, .md (text) — json/text differ from code.
    const tsClass = iconClassFor('index.ts');
    const jsonClass = iconClassFor('package.json');
    const mdClass = iconClassFor('README.md');

    expect(tsClass).toContain('lucide-file-code');
    expect(jsonClass).not.toBe(tsClass);
    expect(mdClass).not.toBe(tsClass);
    expect(jsonClass).not.toBe(mdClass);
  });
});
