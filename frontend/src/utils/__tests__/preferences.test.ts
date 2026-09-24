import { describe, it, expect, beforeEach } from 'vitest';
import {
  loadPreferences,
  savePreferences,
  clampFontSize,
  clampLineHeight,
  defaultPreferences,
  PREFS_KEY,
  DEFAULT_FONT_SIZE,
  DEFAULT_LINE_HEIGHT,
  DEFAULT_CONTEXT_LEVEL,
  SIDEBAR_DEFAULT_WIDTH,
  SIDEBAR_MAX_WIDTH,
  normalizeContextLevel,
  patchPreferences,
} from '../preferences.js';

describe('preferences', () => {
  beforeEach(() => localStorage.clear());

  it('returns defaults when nothing is stored', () => {
    expect(loadPreferences()).toEqual(defaultPreferences());
  });

  it('round-trips a saved preferences blob', () => {
    savePreferences({
      theme: 'catppuccin-mocha',
      fontSize: 16,
      lineHeight: 1.8,
      codeFont: 'fira-code',
      uiFont: 'roboto',
      contextLevel: 20,
      sidebarWidth: 340,
    });
    const loaded = loadPreferences();
    expect(loaded.theme).toBe('catppuccin-mocha');
    expect(loaded.fontSize).toBe(16);
    expect(loaded.lineHeight).toBe(1.8);
    expect(loaded.codeFont).toBe('fira-code');
    expect(loaded.uiFont).toBe('roboto');
    expect(loaded.contextLevel).toBe(20);
    expect(loaded.sidebarWidth).toBe(340);
  });

  it('clamps out-of-range font size and line height on load', () => {
    savePreferences({
      ...defaultPreferences(),
      fontSize: 999,
      lineHeight: 99,
    });
    const loaded = loadPreferences();
    expect(loaded.fontSize).toBe(20);
    expect(loaded.lineHeight).toBe(2.4);
  });

  it('fills line-height default for a blob saved before the field existed', () => {
    // Simulate an older consolidated blob without lineHeight.
    localStorage.setItem(
      PREFS_KEY,
      JSON.stringify({ theme: 'default-dark', fontSize: 14, codeFont: 'sf-mono', uiFont: 'inter' }),
    );
    const loaded = loadPreferences();
    expect(loaded.fontSize).toBe(14);
    expect(loaded.lineHeight).toBe(DEFAULT_LINE_HEIGHT);
  });

  it('migrates legacy single-purpose theme + font-size keys into the blob', () => {
    localStorage.setItem('local-review:theme', 'catppuccin-latte');
    localStorage.setItem('local-review:font-size', '17');

    const loaded = loadPreferences();
    expect(loaded.theme).toBe('catppuccin-latte');
    expect(loaded.fontSize).toBe(17);
    expect(loaded.lineHeight).toBe(DEFAULT_LINE_HEIGHT);

    // The consolidated blob is now persisted and the legacy keys removed.
    expect(localStorage.getItem(PREFS_KEY)).not.toBeNull();
    expect(localStorage.getItem('local-review:theme')).toBeNull();
    expect(localStorage.getItem('local-review:font-size')).toBeNull();
  });

  it('migrates a legacy theme key even when font-size is absent', () => {
    localStorage.setItem('local-review:theme', 'catppuccin-frappe');
    const loaded = loadPreferences();
    expect(loaded.theme).toBe('catppuccin-frappe');
    expect(loaded.fontSize).toBe(DEFAULT_FONT_SIZE);
  });

  it('clampFontSize and clampLineHeight guard non-finite input', () => {
    expect(clampFontSize(Number.NaN)).toBe(DEFAULT_FONT_SIZE);
    expect(clampLineHeight(Number.POSITIVE_INFINITY)).toBe(DEFAULT_LINE_HEIGHT);
  });

  describe('diff context + sidebar width', () => {
    it('defaults and normalizes unknown context levels', () => {
      expect(defaultPreferences().contextLevel).toBe(DEFAULT_CONTEXT_LEVEL);
      expect(normalizeContextLevel('full')).toBe('full');
      expect(normalizeContextLevel('20')).toBe(20);
      expect(normalizeContextLevel('7')).toBe(DEFAULT_CONTEXT_LEVEL);
      expect(normalizeContextLevel(null)).toBe(DEFAULT_CONTEXT_LEVEL);
    });

    it('clamps a stored sidebar width into range', () => {
      savePreferences({ ...defaultPreferences(), sidebarWidth: 9999 });
      expect(loadPreferences().sidebarWidth).toBe(SIDEBAR_MAX_WIDTH);
    });

    it('fills defaults for a blob saved before the fields existed', () => {
      localStorage.setItem(
        PREFS_KEY,
        JSON.stringify({ theme: 'default-dark', fontSize: 14, lineHeight: 1.5 }),
      );
      const loaded = loadPreferences();
      expect(loaded.contextLevel).toBe(DEFAULT_CONTEXT_LEVEL);
      expect(loaded.sidebarWidth).toBe(SIDEBAR_DEFAULT_WIDTH);
    });

    // These two used to live under their own `local-review:*` keys, which
    // cleanExpiredSessions() deleted on every mount.
    it('adopts the legacy standalone keys into an existing blob', () => {
      // A blob written before these fields moved in: the standalone keys are
      // still the only record of the user's choice.
      localStorage.setItem(
        PREFS_KEY,
        JSON.stringify({ theme: 'default-dark', fontSize: 14, lineHeight: 1.5 }),
      );
      localStorage.setItem('local-review:diff-context', 'full');
      localStorage.setItem('local-review:sidebar-width', '400');

      const loaded = loadPreferences();
      expect(loaded.contextLevel).toBe('full');
      expect(loaded.sidebarWidth).toBe(400);
      expect(localStorage.getItem('local-review:diff-context')).toBeNull();
      expect(localStorage.getItem('local-review:sidebar-width')).toBeNull();
    });

    it('migrates the legacy keys when no blob exists at all', () => {
      localStorage.setItem('local-review:diff-context', '50');
      localStorage.setItem('local-review:sidebar-width', '300');

      const loaded = loadPreferences();
      expect(loaded.contextLevel).toBe(50);
      expect(loaded.sidebarWidth).toBe(300);
      expect(localStorage.getItem(PREFS_KEY)).not.toBeNull();
      expect(localStorage.getItem('local-review:diff-context')).toBeNull();
      expect(localStorage.getItem('local-review:sidebar-width')).toBeNull();
    });

    it('patchPreferences leaves the other fields alone', () => {
      savePreferences({ ...defaultPreferences(), theme: 'catppuccin-latte', contextLevel: 20 });
      patchPreferences({ sidebarWidth: 320 });

      const loaded = loadPreferences();
      expect(loaded.sidebarWidth).toBe(320);
      expect(loaded.theme).toBe('catppuccin-latte');
      expect(loaded.contextLevel).toBe(20);
    });
  });
});
