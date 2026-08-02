import type { ThemeId } from '../../shared/types.js';

export interface ShikiThemePair {
  dark: string;
  light: string;
}

/**
 * Maps each app theme to the shiki syntax theme that drives @pierre/diffs
 * highlighting. pierre resolves the entry matching the active `themeType`
 * (dark/light), so both sides are always provided; single-mode app themes
 * point both slots at their own shiki theme.
 */
const THEME_TO_SHIKI: Record<ThemeId, ShikiThemePair> = {
  'default-dark': { dark: 'dark-plus', light: 'dark-plus' },
  'default-light': { dark: 'light-plus', light: 'light-plus' },
  'catppuccin-mocha': { dark: 'catppuccin-mocha', light: 'catppuccin-mocha' },
  'catppuccin-macchiato': { dark: 'catppuccin-macchiato', light: 'catppuccin-macchiato' },
  'catppuccin-frappe': { dark: 'catppuccin-frappe', light: 'catppuccin-frappe' },
  'catppuccin-latte': { dark: 'catppuccin-latte', light: 'catppuccin-latte' },
};

const FALLBACK: ShikiThemePair = THEME_TO_SHIKI['default-dark'];

export function resolveShikiTheme(theme: ThemeId): ShikiThemePair {
  return THEME_TO_SHIKI[theme] ?? FALLBACK;
}
