import type { ThemeId } from './shared/types.js';

export interface ThemeDef {
  id: ThemeId;
  label: string;
  mode: 'dark' | 'light';
}

// Single source of truth for the selector UI and for validating persisted prefs.
export const THEMES: ThemeDef[] = [
  { id: 'default-dark', label: 'Default Dark', mode: 'dark' },
  { id: 'catppuccin-mocha', label: 'Catppuccin Mocha', mode: 'dark' },
  { id: 'catppuccin-macchiato', label: 'Catppuccin Macchiato', mode: 'dark' },
  { id: 'catppuccin-frappe', label: 'Catppuccin Frappé', mode: 'dark' },
  { id: 'default-light', label: 'Default Light', mode: 'light' },
  { id: 'catppuccin-latte', label: 'Catppuccin Latte', mode: 'light' },
];

export const DEFAULT_THEME: ThemeId = 'default-dark';

const THEME_IDS = new Set<string>(THEMES.map((t) => t.id));

/**
 * Coerces an arbitrary persisted value into a valid ThemeId. Migrates the old
 * binary `'dark'`/`'light'` values and falls back to the default for anything
 * unrecognized.
 */
export function normalizeThemeId(value: unknown): ThemeId {
  if (value === 'dark') return 'default-dark';
  if (value === 'light') return 'default-light';
  if (typeof value === 'string' && THEME_IDS.has(value)) return value as ThemeId;
  return DEFAULT_THEME;
}
