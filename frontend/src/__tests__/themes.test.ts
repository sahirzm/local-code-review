import { describe, it, expect } from 'vitest';
import { THEMES, DEFAULT_THEME, normalizeThemeId } from '../themes.js';

describe('THEMES registry', () => {
  it('has unique ids', () => {
    const ids = THEMES.map((t) => t.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it('includes the default theme', () => {
    expect(THEMES.some((t) => t.id === DEFAULT_THEME)).toBe(true);
  });

  it('categorizes every theme as dark or light', () => {
    for (const theme of THEMES) {
      expect(['dark', 'light']).toContain(theme.mode);
    }
  });

  it('ships both dark and light options', () => {
    expect(THEMES.some((t) => t.mode === 'dark')).toBe(true);
    expect(THEMES.some((t) => t.mode === 'light')).toBe(true);
  });
});

describe('normalizeThemeId', () => {
  it('migrates the legacy "dark" value to default-dark', () => {
    expect(normalizeThemeId('dark')).toBe('default-dark');
  });

  it('migrates the legacy "light" value to default-light', () => {
    expect(normalizeThemeId('light')).toBe('default-light');
  });

  it('passes through a known theme id unchanged', () => {
    expect(normalizeThemeId('catppuccin-mocha')).toBe('catppuccin-mocha');
    expect(normalizeThemeId('catppuccin-latte')).toBe('catppuccin-latte');
  });

  it('falls back to the default for an unknown string', () => {
    expect(normalizeThemeId('solarized')).toBe(DEFAULT_THEME);
  });

  it('falls back to the default for non-string input', () => {
    expect(normalizeThemeId(undefined)).toBe(DEFAULT_THEME);
    expect(normalizeThemeId(null)).toBe(DEFAULT_THEME);
    expect(normalizeThemeId(42)).toBe(DEFAULT_THEME);
    expect(normalizeThemeId({})).toBe(DEFAULT_THEME);
  });
});
