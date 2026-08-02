/// <reference types="@vitest/browser/context" />
/// <reference types="@vitest/browser/matchers" />
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-react';
import { App } from './App.js';
import { mockApi, type MockApiHandle } from './test/mock-api.js';

describe('diff surface (browser)', () => {
  let api: MockApiHandle;

  beforeEach(() => {
    localStorage.clear();
    api = mockApi();
  });

  afterEach(() => {
    api.restore();
  });

  it('syntax-highlights diff code via @pierre/diffs (shiki)', async () => {
    const screen = render(<App />);
    await expect.element(screen.getByText('demo-repo')).toBeInTheDocument();

    // Pierre renders into shadow DOM and shiki colors each token through
    // per-token CSS custom properties (--diffs-token-dark / -light). Recurse
    // shadow roots and confirm those tokenized spans exist — the whole point
    // of the swap. (Requires worker-src blob: in the CSP; a blocked worker
    // would leave code un-tokenized and fail this assertion.)
    const countTokenizedSpans = (root: ParentNode): number => {
      let n = root.querySelectorAll('span[style*="--diffs-token"]').length;
      for (const el of root.querySelectorAll('*')) {
        if (el.shadowRoot) n += countTokenizedSpans(el.shadowRoot);
      }
      return n;
    };

    await expect.poll(() => countTokenizedSpans(document), { timeout: 15_000, interval: 250 }).toBeGreaterThan(0);
  });

  it('reflects the selected catppuccin theme in the diff, not just light/dark', async () => {
    const screen = render(<App />);
    await expect.element(screen.getByText('demo-repo')).toBeInTheDocument();

    // Pierre injects the resolved shiki theme's token colors into a <style> in
    // its shadow DOM. Concatenate every shadow-root stylesheet so we can detect
    // when the *named* theme (not just dark/light) actually drives the diff.
    const shadowThemeCSS = (): string => {
      let css = '';
      const walk = (root: ParentNode): void => {
        for (const el of root.querySelectorAll('*')) {
          if (el.shadowRoot) {
            css += el.shadowRoot.textContent ?? '';
            walk(el.shadowRoot);
          }
        }
      };
      walk(document);
      return css;
    };

    // Catppuccin Mocha's signature base #1e1e2e should appear once the theme is
    // synced to the worker pool. Default Dark (pierre-dark) never emits it — so
    // this fails if the diff is stuck on light/dark toggling only.
    // The theme selector lives behind the toolbar gear.
    await screen.getByLabelText('Settings').click();
    const select = screen.getByLabelText('Select color theme');
    await select.selectOptions('Catppuccin Mocha');
    await expect
      .poll(() => shadowThemeCSS().toLowerCase().includes('#1e1e2e'), { timeout: 15_000, interval: 250 })
      .toBe(true);
  });

});

