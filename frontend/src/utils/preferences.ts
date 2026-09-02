import type { UserPreferences } from '../shared/types.js';
import { DEFAULT_THEME, normalizeThemeId } from '../themes.js';
import {
  DEFAULT_CODE_FONT,
  DEFAULT_UI_FONT,
  normalizeCodeFontId,
  normalizeUiFontId,
} from '../fonts.js';

export const PREFS_KEY = 'local-review:preferences';

/// Legacy single-purpose keys migrated into PREFS_KEY on first load, from
/// before display preferences were consolidated into one JSON blob.
const LEGACY_THEME_KEY = 'local-review:theme';
const LEGACY_FONT_SIZE_KEY = 'local-review:font-size';

export const MIN_FONT_SIZE = 10;
export const MAX_FONT_SIZE = 20;
export const DEFAULT_FONT_SIZE = 13;

export const MIN_LINE_HEIGHT = 1.2;
export const MAX_LINE_HEIGHT = 2.4;
// 20/13 reproduces pierre's default row ratio, so existing reviews look identical.
export const DEFAULT_LINE_HEIGHT = Math.round((20 / 13) * 10) / 10;

export function clampFontSize(size: number): number {
  if (!Number.isFinite(size)) return DEFAULT_FONT_SIZE;
  return Math.min(MAX_FONT_SIZE, Math.max(MIN_FONT_SIZE, Math.round(size)));
}

export function clampLineHeight(ratio: number): number {
  if (!Number.isFinite(ratio)) return DEFAULT_LINE_HEIGHT;
  const clamped = Math.min(MAX_LINE_HEIGHT, Math.max(MIN_LINE_HEIGHT, ratio));
  return Math.round(clamped * 10) / 10;
}

export function defaultPreferences(): UserPreferences {
  return {
    theme: DEFAULT_THEME,
    fontSize: DEFAULT_FONT_SIZE,
    lineHeight: DEFAULT_LINE_HEIGHT,
    codeFont: DEFAULT_CODE_FONT,
    uiFont: DEFAULT_UI_FONT,
  };
}

/**
 * Load display preferences from localStorage. Prefers the consolidated JSON
 * blob; if it's absent, migrates any legacy single-purpose keys (theme, font
 * size) into a fresh blob so older installs keep their settings. Falls back to
 * defaults when nothing is stored or storage is unavailable.
 */
export function loadPreferences(): UserPreferences {
  try {
    const raw = localStorage.getItem(PREFS_KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as Partial<UserPreferences>;
      return {
        theme: normalizeThemeId(parsed.theme),
        fontSize: clampFontSize(parsed.fontSize ?? DEFAULT_FONT_SIZE),
        lineHeight: clampLineHeight(parsed.lineHeight ?? DEFAULT_LINE_HEIGHT),
        codeFont: normalizeCodeFontId(parsed.codeFont),
        uiFont: normalizeUiFontId(parsed.uiFont),
      };
    }

    // No consolidated blob — migrate legacy single-purpose keys if present.
    const legacyTheme = localStorage.getItem(LEGACY_THEME_KEY);
    const legacyFontSize = localStorage.getItem(LEGACY_FONT_SIZE_KEY);
    if (legacyTheme !== null || legacyFontSize !== null) {
      const migrated: UserPreferences = {
        ...defaultPreferences(),
        theme: normalizeThemeId(legacyTheme),
        fontSize: clampFontSize(Number(legacyFontSize ?? DEFAULT_FONT_SIZE)),
      };
      // Persist the consolidated blob and drop the legacy keys.
      savePreferences(migrated);
      try {
        localStorage.removeItem(LEGACY_THEME_KEY);
        localStorage.removeItem(LEGACY_FONT_SIZE_KEY);
      } catch { /* ignore */ }
      return migrated;
    }
  } catch { /* ignore */ }
  return defaultPreferences();
}

export function savePreferences(prefs: UserPreferences): void {
  try {
    localStorage.setItem(PREFS_KEY, JSON.stringify(prefs));
  } catch { /* best effort */ }
}
