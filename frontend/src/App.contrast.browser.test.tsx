/// <reference types="@vitest/browser/context" />
/// <reference types="@vitest/browser/matchers" />
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-react';
import { App } from './App.js';
import { THEMES } from './themes.js';
import { mockApi, type MockApiHandle } from './test/mock-api.js';

/**
 * Six themes redefine every semantic token, so a palette edit in one of them
 * can quietly drop a filled button or a body-text pair below a readable ratio.
 * These measure the *resolved* colors in a real browser — which is the only way
 * to evaluate the `color-mix()`-derived tokens — and hold the whole set to a
 * floor.
 */

/** Relative luminance from a computed color string. */
function luminance(color: string): number {
  // Computed values arrive as `rgb(0-255 …)`, or as `color(srgb 0-1 …)` when
  // color-mix() was involved.
  const isUnitScale = color.trim().startsWith('color(');
  const channels = (color.match(/-?\d+(\.\d+)?/g) ?? []).slice(0, 3).map(Number);
  expect(channels).toHaveLength(3);
  const linear = channels.map((raw) => {
    const c = isUnitScale ? raw : raw / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2];
}

function contrast(a: string, b: string): number {
  const [light, dark] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (light + 0.05) / (dark + 0.05);
}

/** WCAG 1.4.3 for body text. */
const TEXT_FLOOR = 4.5;
/** WCAG 1.4.11 for icons and control boundaries. */
const NON_TEXT_FLOOR = 3;

/** Text pairs: [label, background token, foreground token]. */
const TEXT_PAIRS: [string, string, string][] = [
  ['body text', '--color-bg', '--color-text'],
  ['headings', '--color-bg', '--color-text-heading'],
  ['panel text', '--color-panel', '--color-text'],
  ['dim text', '--color-bg', '--color-text-dim'],
  ['muted text', '--color-bg', '--color-text-muted'],
  ['muted on panel', '--color-panel', '--color-text-muted'],
  ['subtle text', '--color-bg', '--color-text-subtle'],
  // Filled controls, each with the ink role that actually sits on it.
  ['accent fill', '--color-accent', '--color-on-accent'],
  ['accent fill hover', '--color-accent-hover', '--color-on-accent'],
  ['success fill', '--color-success', '--color-on-success'],
  ['success fill hover', '--color-success-strong', '--color-on-success'],
  ['danger fill', '--color-danger', '--color-on-danger'],
  ['danger fill hover', '--color-danger-strong', '--color-on-danger'],
  ['muted pill', '--color-text-muted', '--color-on-muted'],
  // Selection has to stay readable while also being distinguishable (below).
  ['text on selection', '--color-selection-bg', '--color-text'],
  ['selection label', '--color-selection-bg', '--color-selection-text'],
];

/**
 * Non-text pairs held only to the 3:1 UI-component floor: colors that carry
 * meaning on their own (status dots, the review tick, the focus ring, links).
 *
 * Deliberately excluded: `--color-border*`. Those are separators and control
 * outlines that sit at 1.6-2.0:1 against the surfaces they divide. 1.4.11 asks
 * for 3:1 on boundaries *needed to identify* a control, and every control here
 * is identified by its own fill and label, not by its outline. Raising them all
 * would mean redrawing the visual language, not fixing a defect.
 */
const NON_TEXT_PAIRS: [string, string, string][] = [
  ['focus ring', '--color-bg', '--color-accent'],
  ['link on bg', '--color-bg', '--color-link'],
  ['warning on bg', '--color-bg', '--color-warning'],
  ['success on bg', '--color-bg', '--color-success'],
  ['danger on bg', '--color-bg', '--color-danger'],
];

describe('theme contrast (browser)', () => {
  let api: MockApiHandle;

  beforeEach(() => {
    localStorage.clear();
    api = mockApi();
  });

  afterEach(() => {
    api.restore();
    document.documentElement.setAttribute('data-theme', 'default-dark');
  });

  it('meets the contrast floors in every theme', async () => {
    const screen = render(<App />);
    await expect.element(screen.getByText('demo-repo')).toBeInTheDocument();

    // Resolving through a real element is what expands color-mix() and var().
    const probe = document.createElement('div');
    document.body.appendChild(probe);
    const resolve = (token: string): string => {
      probe.style.color = '';
      probe.style.color = `var(${token})`;
      return getComputedStyle(probe).color;
    };

    const failures: string[] = [];
    for (const theme of THEMES) {
      document.documentElement.setAttribute('data-theme', theme.id);
      const check = (pairs: [string, string, string][], floor: number): void => {
        for (const [label, bg, fg] of pairs) {
          const ratio = contrast(resolve(bg), resolve(fg));
          if (ratio < floor) {
            failures.push(`${theme.id} · ${label}: ${ratio.toFixed(2)} < ${floor}`);
          }
        }
      };
      check(TEXT_PAIRS, TEXT_FLOOR);
      check(NON_TEXT_PAIRS, NON_TEXT_FLOOR);
    }
    probe.remove();

    expect(failures).toEqual([]);
  });

  it('keeps a selected row distinguishable from a hovered one', async () => {
    const screen = render(<App />);
    await expect.element(screen.getByText('demo-repo')).toBeInTheDocument();

    const probe = document.createElement('div');
    document.body.appendChild(probe);
    const resolve = (token: string): string => {
      probe.style.color = '';
      probe.style.color = `var(${token})`;
      return getComputedStyle(probe).color;
    };

    // These tokens were byte-identical in four of six themes, which made a
    // selected file indistinguishable from a hovered one.
    const tooClose: string[] = [];
    for (const theme of THEMES) {
      document.documentElement.setAttribute('data-theme', theme.id);
      const selection = resolve('--color-selection-bg');
      const hover = resolve('--color-panel-hover');
      if (selection === hover) tooClose.push(`${theme.id}: identical (${selection})`);
    }
    probe.remove();

    expect(tooClose).toEqual([]);
  });
});
