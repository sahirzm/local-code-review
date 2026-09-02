import { describe, it, expect } from 'vitest';
import {
  clampWidth,
  SIDEBAR_MIN_WIDTH,
  SIDEBAR_MAX_WIDTH,
  SIDEBAR_DEFAULT_WIDTH,
} from '../useSidebarWidth';

describe('clampWidth', () => {
  it('passes through an in-range width', () => {
    expect(clampWidth(300)).toBe(300);
  });

  it('clamps below the minimum', () => {
    expect(clampWidth(50)).toBe(SIDEBAR_MIN_WIDTH);
  });

  it('clamps above the maximum', () => {
    expect(clampWidth(9999)).toBe(SIDEBAR_MAX_WIDTH);
  });

  it('rounds fractional widths', () => {
    expect(clampWidth(321.7)).toBe(322);
  });

  it('falls back to the default for non-finite input', () => {
    expect(clampWidth(Number.NaN)).toBe(SIDEBAR_DEFAULT_WIDTH);
  });
});
