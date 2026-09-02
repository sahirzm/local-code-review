// Pre-paint preference bootstrap. Loaded from <head> before the app bundle so
// the persisted theme, font, and font-size are applied to <html> before first
// paint — avoiding a flash of the default theme while React mounts and reads
// localStorage. The CSP forbids inline scripts (script-src 'self'), so this
// must be a real module served from the origin rather than an inline <script>.
//
// It intentionally duplicates a small amount of logic from utils/preferences.ts
// and fonts.ts rather than importing them, to stay tiny and dependency-free on
// the critical path. The app still re-applies everything on mount, so any drift
// here is corrected immediately and only affects the very first paint.

const PREFS_KEY = 'local-review:preferences';
const DEFAULT_FONT_SIZE = 13;
const DEFAULT_LINE_HEIGHT = Math.round((20 / 13) * 10) / 10;

const VALID_THEMES = new Set([
  'default-dark',
  'default-light',
  'catppuccin-mocha',
  'catppuccin-macchiato',
  'catppuccin-frappe',
  'catppuccin-latte',
]);

try {
  const raw = localStorage.getItem(PREFS_KEY);
  if (raw) {
    const prefs = JSON.parse(raw) as {
      theme?: string;
      fontSize?: number;
      lineHeight?: number;
    };
    const root = document.documentElement;

    if (typeof prefs.theme === 'string' && VALID_THEMES.has(prefs.theme)) {
      root.setAttribute('data-theme', prefs.theme);
    }

    const fontSize = Number.isFinite(prefs.fontSize) ? (prefs.fontSize as number) : DEFAULT_FONT_SIZE;
    const lineHeight = Number.isFinite(prefs.lineHeight)
      ? (prefs.lineHeight as number)
      : DEFAULT_LINE_HEIGHT;
    root.style.setProperty('--diffs-font-size', `${fontSize}px`);
    root.style.setProperty('--diffs-line-height', `${Math.round(fontSize * lineHeight)}px`);
  }
} catch {
  // localStorage unavailable or malformed blob — the app applies defaults on mount.
}
