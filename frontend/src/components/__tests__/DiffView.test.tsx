import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { DiffView } from '../DiffView.js';
import { ReviewStoreProvider } from '../../hooks/useReviewStore.js';
import type { ParsedFileDiff } from '../../shared/types.js';
import type { ReactNode } from 'react';

function wrapper({ children }: { children: ReactNode }) {
  return <ReviewStoreProvider>{children}</ReviewStoreProvider>;
}

const SYNTAX_THEME = { dark: 'dark-plus', light: 'light-plus' };

function file(path: string): ParsedFileDiff {
  return {
    oldPath: path,
    newPath: path,
    status: 'modified',
    additions: 1,
    deletions: 0,
    isBinary: false,
    isLarge: false,
    rawPatch: [
      `diff --git a/${path} b/${path}`,
      `--- a/${path}`,
      `+++ b/${path}`,
      '@@ -1 +1,2 @@',
      ' a',
      '+b',
      '',
    ].join('\n'),
    hunks: [
      {
        oldStart: 1,
        oldLines: 1,
        newStart: 1,
        newLines: 2,
        content: '@@ -1 +1,2 @@',
        changes: [
          { type: 'normal', content: 'a', oldLineNumber: 1, newLineNumber: 1 },
          { type: 'insert', content: 'b', newLineNumber: 2 },
        ],
      },
    ],
  };
}

describe('DiffView', () => {
  it('renders a loading skeleton when files is null', () => {
    render(<DiffView files={null} currentIndex={0} viewType="unified" themeType="dark" syntaxTheme={SYNTAX_THEME} fontSize={13} lineHeight={1.5} />, { wrapper });
    expect(screen.getByRole('status', { name: 'Loading diffs' })).toBeTruthy();
  });

  it('renders an empty state when there are no files', () => {
    render(<DiffView files={[]} currentIndex={0} viewType="unified" themeType="dark" syntaxTheme={SYNTAX_THEME} fontSize={13} lineHeight={1.5} />, { wrapper });
    expect(screen.getByText('No files changed')).toBeTruthy();
  });

  it('renders only the file at currentIndex (single-file view)', () => {
    render(
      <DiffView
        files={[file('a.ts'), file('b.ts'), file('c.ts')]}
        currentIndex={1}
        viewType="unified"
        themeType="dark"
        syntaxTheme={SYNTAX_THEME}
        fontSize={13}
        lineHeight={1.5}
      />,
      { wrapper },
    );
    // Exactly one file surface is mounted, and it is the selected one.
    expect(screen.getByText('b.ts')).toBeTruthy();
    expect(screen.queryByText('a.ts')).toBeNull();
    expect(screen.queryByText('c.ts')).toBeNull();
  });

  it('clamps an out-of-range currentIndex to the last file', () => {
    render(
      <DiffView
        files={[file('a.ts'), file('b.ts')]}
        currentIndex={99}
        viewType="unified"
        themeType="dark"
        syntaxTheme={SYNTAX_THEME}
        fontSize={13}
        lineHeight={1.5}
      />,
      { wrapper },
    );
    expect(screen.getByText('b.ts')).toBeTruthy();
    expect(screen.queryByText('a.ts')).toBeNull();
  });

  it('remounts the file surface when the font size changes', () => {
    const props = {
      files: [file('a.ts')],
      currentIndex: 0,
      viewType: 'unified' as const,
      themeType: 'dark' as const,
      syntaxTheme: SYNTAX_THEME,
      lineHeight: 1.5,
    };
    const { container, rerender } = render(
      <DiffView {...props} fontSize={13} />,
      { wrapper },
    );
    const before = container.querySelector('.file-diff');
    expect(before).not.toBeNull();

    // A font-size change must remount FileDiff (new React key) so @pierre/diffs
    // re-measures its cached row layout at the new size.
    rerender(<DiffView {...props} fontSize={18} />);
    const after = container.querySelector('.file-diff');
    expect(after).not.toBeNull();
    expect(after).not.toBe(before);
  });
});
