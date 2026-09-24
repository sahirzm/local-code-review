import type { DiffContextLevel, UserPreferences } from '../shared/types.js';
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
/// Diff context and sidebar width used to live under their own keys, which
/// `cleanExpiredSessions()` deleted on every mount because they looked like
/// stale review sessions. Read once, then folded into PREFS_KEY.
const LEGACY_CONTEXT_KEY = 'local-review:diff-context';
const LEGACY_SIDEBAR_WIDTH_KEY = 'local-review:sidebar-width';

export const MIN_FONT_SIZE = 10;
export const MAX_FONT_SIZE = 20;
export const DEFAULT_FONT_SIZE = 13;

export const MIN_LINE_HEIGHT = 1.2;
export const MAX_LINE_HEIGHT = 2.4;
// 20/13 reproduces pierre's default row ratio, so existing reviews look identical.
export const DEFAULT_LINE_HEIGHT = Math.round((20 / 13) * 10) / 10;

/** Context-line levels offered in the toolbar. */
export const CONTEXT_LEVELS = [5, 10, 20, 50, 'full'] as const satisfies readonly DiffContextLevel[];
export const DEFAULT_CONTEXT_LEVEL: DiffContextLevel = 5;

export const SIDEBAR_MIN_WIDTH = 220;
export const SIDEBAR_MAX_WIDTH = 640;
export const SIDEBAR_DEFAULT_WIDTH = 260;

export function normalizeContextLevel(value: unknown): DiffContextLevel {
  if (value === 'full') return 'full';
  const n = Number(value);
  return (CONTEXT_LEVELS as readonly unknown[]).includes(n)
    ? (n as DiffContextLevel)
    : DEFAULT_CONTEXT_LEVEL;
}

export function clampSidebarWidth(px: number): number {
  if (!Number.isFinite(px)) return SIDEBAR_DEFAULT_WIDTH;
  return Math.min(SIDEBAR_MAX_WIDTH, Math.max(SIDEBAR_MIN_WIDTH, Math.round(px)));
}

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
    contextLevel: DEFAULT_CONTEXT_LEVEL,
    sidebarWidth: SIDEBAR_DEFAULT_WIDTH,
  };
}

/** Reads the legacy standalone keys, if any survived, and removes them. */
function takeLegacyKey(key: string): string | null {
  try {
    const value = localStorage.getItem(key);
    if (value !== null) localStorage.removeItem(key);
    return value;
  } catch {
    return null;
  }
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
      // A blob written before these two moved in won't carry them; fall back to
      // the legacy key (consuming it) before the default.
      const contextLevel = parsed.contextLevel ?? takeLegacyKey(LEGACY_CONTEXT_KEY) ?? DEFAULT_CONTEXT_LEVEL;
      const sidebarWidth = parsed.sidebarWidth
        ?? Number(takeLegacyKey(LEGACY_SIDEBAR_WIDTH_KEY) ?? SIDEBAR_DEFAULT_WIDTH);
      return {
        theme: normalizeThemeId(parsed.theme),
        fontSize: clampFontSize(parsed.fontSize ?? DEFAULT_FONT_SIZE),
        lineHeight: clampLineHeight(parsed.lineHeight ?? DEFAULT_LINE_HEIGHT),
        codeFont: normalizeCodeFontId(parsed.codeFont),
        uiFont: normalizeUiFontId(parsed.uiFont),
        contextLevel: normalizeContextLevel(contextLevel),
        sidebarWidth: clampSidebarWidth(sidebarWidth),
      };
    }

    // No consolidated blob — migrate legacy single-purpose keys if present.
    const legacyTheme = localStorage.getItem(LEGACY_THEME_KEY);
    const legacyFontSize = localStorage.getItem(LEGACY_FONT_SIZE_KEY);
    const legacyContext = localStorage.getItem(LEGACY_CONTEXT_KEY);
    const legacySidebarWidth = localStorage.getItem(LEGACY_SIDEBAR_WIDTH_KEY);
    if (legacyTheme !== null || legacyFontSize !== null || legacyContext !== null || legacySidebarWidth !== null) {
      const migrated: UserPreferences = {
        ...defaultPreferences(),
        theme: normalizeThemeId(legacyTheme),
        fontSize: clampFontSize(Number(legacyFontSize ?? DEFAULT_FONT_SIZE)),
        contextLevel: normalizeContextLevel(legacyContext),
        sidebarWidth: clampSidebarWidth(Number(legacySidebarWidth ?? SIDEBAR_DEFAULT_WIDTH)),
      };
      // Persist the consolidated blob and drop the legacy keys.
      savePreferences(migrated);
      try {
        localStorage.removeItem(LEGACY_THEME_KEY);
        localStorage.removeItem(LEGACY_FONT_SIZE_KEY);
        localStorage.removeItem(LEGACY_CONTEXT_KEY);
        localStorage.removeItem(LEGACY_SIDEBAR_WIDTH_KEY);
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

/**
 * Merge one field into the stored blob. Used by preferences that are owned by a
 * hook rather than App's render state (sidebar width), so they cannot clobber
 * the fields they don't know about.
 */
export function patchPreferences(patch: Partial<UserPreferences>): void {
  savePreferences({ ...loadPreferences(), ...patch });
}
