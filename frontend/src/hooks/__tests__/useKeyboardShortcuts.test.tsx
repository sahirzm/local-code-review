import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { renderHook } from '@testing-library/react';
import { useKeyboardShortcuts, SHORTCUT_LIST } from '../useKeyboardShortcuts.js';

type Actions = Parameters<typeof useKeyboardShortcuts>[0];

function makeActions(): Record<keyof Actions, ReturnType<typeof vi.fn>> {
  return {
    nextFile: vi.fn(),
    prevFile: vi.fn(),
    nextComment: vi.fn(),
    prevComment: vi.fn(),
    addComment: vi.fn(),
    toggleViewMode: vi.fn(),
    closeForm: vi.fn(),
    toggleHelp: vi.fn(),
  };
}

function press(key: string): void {
  document.dispatchEvent(new KeyboardEvent('keydown', { key }));
}

describe('useKeyboardShortcuts', () => {
  let actions: ReturnType<typeof makeActions>;

  beforeEach(() => {
    actions = makeActions();
  });

  afterEach(() => {
    document.body.innerHTML = '';
  });

  it('maps each navigation key to its action', () => {
    renderHook(() => useKeyboardShortcuts(actions));
    const cases: Array<[string, keyof Actions]> = [
      ['n', 'nextFile'],
      ['p', 'prevFile'],
      ['j', 'nextComment'],
      ['k', 'prevComment'],
      ['c', 'addComment'],
      ['d', 'toggleViewMode'],
      ['?', 'toggleHelp'],
    ];
    for (const [key, action] of cases) {
      press(key);
      expect(actions[action]).toHaveBeenCalledTimes(1);
    }
  });

  it('always fires closeForm on Escape', () => {
    renderHook(() => useKeyboardShortcuts(actions));
    press('Escape');
    expect(actions.closeForm).toHaveBeenCalledTimes(1);
  });

  it('ignores unmapped keys', () => {
    renderHook(() => useKeyboardShortcuts(actions));
    press('x');
    for (const fn of Object.values(actions)) {
      expect(fn).not.toHaveBeenCalled();
    }
  });

  it('suppresses shortcuts while a text input is focused', () => {
    const input = document.createElement('input');
    document.body.appendChild(input);
    input.focus();

    renderHook(() => useKeyboardShortcuts(actions));
    press('n');
    expect(actions.nextFile).not.toHaveBeenCalled();
  });

  it('still fires Escape while a text input is focused', () => {
    const textarea = document.createElement('textarea');
    document.body.appendChild(textarea);
    textarea.focus();

    renderHook(() => useKeyboardShortcuts(actions));
    press('Escape');
    expect(actions.closeForm).toHaveBeenCalledTimes(1);
  });

  it('uses the latest action callbacks without rebinding', () => {
    const { rerender } = renderHook(({ a }) => useKeyboardShortcuts(a), {
      initialProps: { a: actions as Actions },
    });
    const replacement = makeActions();
    rerender({ a: replacement as Actions });

    press('n');
    expect(actions.nextFile).not.toHaveBeenCalled();
    expect(replacement.nextFile).toHaveBeenCalledTimes(1);
  });

  it('detaches its listener on unmount', () => {
    const { unmount } = renderHook(() => useKeyboardShortcuts(actions));
    unmount();
    press('n');
    expect(actions.nextFile).not.toHaveBeenCalled();
  });
});

describe('SHORTCUT_LIST', () => {
  it('documents a key and description for every entry', () => {
    expect(SHORTCUT_LIST.length).toBeGreaterThan(0);
    for (const entry of SHORTCUT_LIST) {
      expect(entry.key).toBeTruthy();
      expect(entry.description).toBeTruthy();
    }
  });
});
