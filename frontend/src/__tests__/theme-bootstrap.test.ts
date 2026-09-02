import { describe, it, expect, beforeEach, vi } from 'vitest';

const PREFS_KEY = 'local-review:preferences';

/**
 * The bootstrap applies preferences as a side effect on import. Each test
 * seeds localStorage, resets the module registry, then imports it fresh so the
 * side effect re-runs against the current document/localStorage.
 */
async function runBootstrap(): Promise<void> {
  vi.resetModules();
  await import('../theme-bootstrap.js');
}

describe('theme-bootstrap', () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.removeAttribute('data-theme');
    document.documentElement.style.removeProperty('--diffs-font-size');
    document.documentElement.style.removeProperty('--diffs-line-height');
  });

  it('applies the persisted theme and font vars before paint', async () => {
    localStorage.setItem(
      PREFS_KEY,
      JSON.stringify({ theme: 'catppuccin-mocha', fontSize: 16, lineHeight: 1.8 }),
    );
    await runBootstrap();

    const root = document.documentElement;
    expect(root.getAttribute('data-theme')).toBe('catppuccin-mocha');
    expect(root.style.getPropertyValue('--diffs-font-size')).toBe('16px');
    expect(root.style.getPropertyValue('--diffs-line-height')).toBe(`${Math.round(16 * 1.8)}px`);
  });

  it('ignores an unknown theme id', async () => {
    localStorage.setItem(PREFS_KEY, JSON.stringify({ theme: 'not-a-theme', fontSize: 14 }));
    await runBootstrap();
    expect(document.documentElement.getAttribute('data-theme')).toBeNull();
    // Font size is still applied even when the theme is invalid.
    expect(document.documentElement.style.getPropertyValue('--diffs-font-size')).toBe('14px');
  });

  it('does nothing when no preferences are stored', async () => {
    await runBootstrap();
    expect(document.documentElement.getAttribute('data-theme')).toBeNull();
    expect(document.documentElement.style.getPropertyValue('--diffs-font-size')).toBe('');
  });

  it('tolerates a malformed blob', async () => {
    localStorage.setItem(PREFS_KEY, '{not json');
    await runBootstrap();
    expect(document.documentElement.getAttribute('data-theme')).toBeNull();
  });
});
