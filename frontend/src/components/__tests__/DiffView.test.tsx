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
    render(<DiffView files={null} viewType="unified" themeType="dark" syntaxTheme={SYNTAX_THEME} />, { wrapper });
    expect(screen.getByRole('status', { name: 'Loading diffs' })).toBeTruthy();
  });

  it('renders an empty state when there are no files', () => {
    render(<DiffView files={[]} viewType="unified" themeType="dark" syntaxTheme={SYNTAX_THEME} />, { wrapper });
    expect(screen.getByText('No files changed')).toBeTruthy();
  });

  it('renders a virtualized scroll container for files', () => {
    const { container } = render(
      <DiffView files={[file('a.ts')]} viewType="unified" themeType="dark" syntaxTheme={SYNTAX_THEME} />,
      { wrapper },
    );
    expect(container.querySelector('.diff-view-scroll')).not.toBeNull();
    expect(container.querySelector('.diff-view-inner')).not.toBeNull();
  });
});
