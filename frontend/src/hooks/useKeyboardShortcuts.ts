import { useEffect, useCallback, useRef } from 'react';

interface ShortcutActions {
  nextFile: () => void;
  prevFile: () => void;
  nextComment: () => void;
  prevComment: () => void;
  addComment: () => void;
  toggleViewMode: () => void;
  closeForm: () => void;
  toggleHelp: () => void;
}

function isInputFocused(): boolean {
  const el = document.activeElement;
  if (!el) return false;
  const tag = el.tagName.toLowerCase();
  return tag === 'textarea' || tag === 'input' || (el as HTMLElement).isContentEditable;
}

export function useKeyboardShortcuts(actions: ShortcutActions): void {
  const actionsRef = useRef(actions);
  actionsRef.current = actions;

  const handler = useCallback((e: KeyboardEvent) => {
    // Escape always works (closes forms/modals)
    if (e.key === 'Escape') {
      actionsRef.current.closeForm();
      return;
    }

    // All other shortcuts disabled when input focused
    if (isInputFocused()) return;

    switch (e.key) {
      case 'n':
        actionsRef.current.nextFile();
        break;
      case 'p':
        actionsRef.current.prevFile();
        break;
      case 'j':
        actionsRef.current.nextComment();
        break;
      case 'k':
        actionsRef.current.prevComment();
        break;
      case 'c':
        actionsRef.current.addComment();
        break;
      case 'd':
        actionsRef.current.toggleViewMode();
        break;
      case '?':
        actionsRef.current.toggleHelp();
        break;
      default:
        return;
    }
  }, []);

  useEffect(() => {
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  }, [handler]);
}

export const SHORTCUT_LIST: ReadonlyArray<{ key: string; description: string }> = [
  { key: 'n', description: 'Next file' },
  { key: 'p', description: 'Previous file' },
  { key: 'j', description: 'Next comment' },
  { key: 'k', description: 'Previous comment' },
  { key: 'c', description: 'Add comment on focused line' },
  { key: 'd', description: 'Toggle split/unified view' },
  { key: 'Esc', description: 'Close comment form / modal' },
  { key: '?', description: 'Toggle keyboard shortcuts help' },
];
