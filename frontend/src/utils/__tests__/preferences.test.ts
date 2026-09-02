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
    });
    const loaded = loadPreferences();
    expect(loaded.theme).toBe('catppuccin-mocha');
    expect(loaded.fontSize).toBe(16);
    expect(loaded.lineHeight).toBe(1.8);
    expect(loaded.codeFont).toBe('fira-code');
    expect(loaded.uiFont).toBe('roboto');
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
});
