import { describe, it, expect } from 'vitest';
import { resolveShikiTheme } from '../shikiTheme.js';
import { THEMES } from '../../../themes.js';

describe('resolveShikiTheme', () => {
  it('maps every app theme to a shiki theme pair', () => {
    for (const theme of THEMES) {
      const pair = resolveShikiTheme(theme.id);
      expect(pair.dark).toBeTruthy();
      expect(pair.light).toBeTruthy();
    }
  });

  it('maps the default dark/light themes to the VS Code plus themes', () => {
    expect(resolveShikiTheme('default-dark')).toEqual({ dark: 'dark-plus', light: 'dark-plus' });
    expect(resolveShikiTheme('default-light')).toEqual({ dark: 'light-plus', light: 'light-plus' });
  });

  it('maps catppuccin variants to their shiki equivalents', () => {
    expect(resolveShikiTheme('catppuccin-mocha').dark).toBe('catppuccin-mocha');
    expect(resolveShikiTheme('catppuccin-latte').light).toBe('catppuccin-latte');
  });

  it('falls back to the default dark pair for an unknown theme', () => {
    // @ts-expect-error deliberately passing an invalid theme id
    expect(resolveShikiTheme('does-not-exist')).toEqual({ dark: 'dark-plus', light: 'dark-plus' });
  });
});
