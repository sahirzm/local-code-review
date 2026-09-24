/// <reference types="@vitest/browser/context" />
/// <reference types="@vitest/browser/matchers" />
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-react';
import { App } from './App.js';
import { mockApi, type MockApiHandle } from './test/mock-api.js';

/**
 * The sidebar's filter pills were capsules while every button in the main column
 * was a 4px rectangle, so the two halves of the app looked like two products.
 * Controls now share one radius; this measures the resolved value rather than
 * trusting the token, since any later literal would silently drift again.
 */
describe('control shape (browser)', () => {
  let api: MockApiHandle;

  beforeEach(() => {
    localStorage.clear();
    api = mockApi();
  });

  afterEach(() => {
    api.restore();
  });

  /** Every selector here is a thing you click or type into. */
  const CONTROLS = [
    '.header-actions .toolbar-btn',
    '.header-actions .context-select',
    '.sidebar-search',
    '.segmented',
    '.sidebar-quick-filters .filter-pill',
    '.sidebar-overall-btn',
    '.diff-end-bar .btn',
  ];

  function radiusOf(token: string): string {
    const probe = document.createElement('div');
    document.body.appendChild(probe);
    probe.style.borderRadius = `var(${token})`;
    const value = getComputedStyle(probe).borderTopLeftRadius;
    probe.remove();
    return value;
  }

  it('gives every control in both columns the same corner radius', async () => {
    const screen = render(<App />);
    await expect.element(screen.getByText('demo-repo')).toBeInTheDocument();

    const expected = radiusOf('--radius-md');
    expect(expected).toBe('6px');

    const seen: Record<string, string> = {};
    for (const sel of CONTROLS) {
      const el = document.querySelector(sel);
      expect(el, `missing control: ${sel}`).not.toBeNull();
      seen[sel] = getComputedStyle(el as Element).borderTopLeftRadius;
    }

    expect(seen).toEqual(Object.fromEntries(CONTROLS.map((s) => [s, expected])));
  });

  it('keeps the status filter a single bar: the container owns the border', async () => {
    const screen = render(<App />);
    await expect.element(screen.getByText('demo-repo')).toBeInTheDocument();

    const segments = Array.from(document.querySelectorAll('.segmented-btn'));
    expect(segments.length).toBe(5);

    for (const seg of segments) {
      const cs = getComputedStyle(seg);
      // Square segments: rounding them individually would make the group read
      // as five separate buttons crammed together.
      expect(cs.borderTopLeftRadius).toBe('0px');
      expect(cs.borderTopWidth).toBe('0px');
      expect(cs.borderRightWidth).toBe('0px');
    }
    // Dividers, not outlines: only the segments after the first have one.
    expect(getComputedStyle(segments[0]).borderLeftWidth).toBe('0px');
    for (const seg of segments.slice(1)) {
      expect(getComputedStyle(seg).borderLeftWidth).toBe('1px');
    }
  });

  it('shows every status label in full at the narrowest sidebar', async () => {
    const screen = render(<App />);
    await expect.element(screen.getByText('demo-repo')).toBeInTheDocument();

    // A `flex-basis: 0` here would equalise the segments and truncate the
    // longest label ("Renamed") while "All" sat in whitespace.
    const sidebar = document.querySelector('.sidebar') as HTMLElement;
    sidebar.style.setProperty('--sidebar-width', '220px');

    for (const seg of Array.from(document.querySelectorAll('.segmented-btn'))) {
      expect(
        seg.scrollWidth,
        `"${seg.textContent}" is clipped (${seg.scrollWidth} > ${seg.clientWidth})`,
      ).toBeLessThanOrEqual(seg.clientWidth);
    }
  });
});
