import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { FileDiff } from '../FileDiff.js';
import { ReviewStoreProvider } from '../../hooks/useReviewStore.js';
import type { ParsedFileDiff } from '../../shared/types.js';
import type { ReactNode } from 'react';

function wrapper({ children }: { children: ReactNode }) {
  return <ReviewStoreProvider>{children}</ReviewStoreProvider>;
}

const SYNTAX_THEME = { dark: 'dark-plus', light: 'light-plus' };

function fileDiff(overrides: Partial<ParsedFileDiff> = {}): ParsedFileDiff {
  return {
    oldPath: 'a.ts',
    newPath: 'a.ts',
    hunks: [],
    status: 'modified',
    additions: 1,
    deletions: 1,
    isBinary: false,
    isLarge: false,
    rawPatch: [
      'diff --git a/a.ts b/a.ts',
      '--- a/a.ts',
      '+++ b/a.ts',
      '@@ -1,2 +1,2 @@',
      ' a',
      '-old',
      '+new',
      '',
    ].join('\n'),
    ...overrides,
  };
}

function renderFileDiff(file: ParsedFileDiff) {
  return render(
    <FileDiff file={file} viewType="unified" themeType="dark" syntaxTheme={SYNTAX_THEME} />,
    { wrapper },
  );
}

describe('FileDiff', () => {
  it('renders the file path header', () => {
    renderFileDiff(fileDiff());
    expect(screen.getByText('a.ts')).toBeTruthy();
  });

  it('shows a binary-file message instead of a diff for binary files', () => {
    renderFileDiff(fileDiff({ isBinary: true }));
    expect(screen.getByText('Binary file changed')).toBeTruthy();
  });

  it('collapses large files behind a show-diff button', () => {
    const { container } = renderFileDiff(fileDiff({ isLarge: true, additions: 9000, deletions: 3000 }));
    const showBtn = container.querySelector('.show-diff-btn');
    expect(showBtn).not.toBeNull();
    expect(showBtn?.textContent).toContain('12000 lines');
  });

  it('renders the pierre diff surface for a non-binary file', () => {
    const { container } = renderFileDiff(fileDiff());
    // Pierre mounts its <diffs-*> web component host inside .file-diff.
    expect(container.querySelector('.file-diff')).not.toBeNull();
    expect(container.querySelector('.binary-message')).toBeNull();
  });
});
