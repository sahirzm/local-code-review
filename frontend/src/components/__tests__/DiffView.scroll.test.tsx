import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render } from '@testing-library/react';
import { createRef } from 'react';
import type { ReactNode } from 'react';
import { DiffView, type DiffViewHandle } from '../DiffView.js';
import { ReviewStoreProvider } from '../../hooks/useReviewStore.js';
import type { ParsedFileDiff } from '../../shared/types.js';

// Capture the scrollToIndex spy from a mocked virtualizer so we can assert the
// tree-click handler always drives it — the regression guard for the old
// `if (!el)` guard that made scrollToFile a no-op for already-rendered rows.
const scrollToIndex = vi.fn();
let renderedCount = 0;

vi.mock('@tanstack/react-virtual', () => ({
  useVirtualizer: () => ({
    getTotalSize: () => 1200,
    // Report every file as already rendered — the exact condition under which
    // the old `if (!el)` guard skipped scrolling.
    getVirtualItems: () =>
      Array.from({ length: renderedCount }, (_, index) => ({ index, key: index, start: index * 400 })),
    scrollToIndex,
    measureElement: () => undefined,
  }),
}));

function wrapper({ children }: { children: ReactNode }) {
  return <ReviewStoreProvider>{children}</ReviewStoreProvider>;
}

function file(path: string): ParsedFileDiff {
  return {
    oldPath: path, newPath: path, status: 'modified', additions: 1, deletions: 0,
    isBinary: false, isLarge: false, rawPatch: '',
    hunks: [],
  };
}

const SYNTAX_THEME = { dark: 'dark-plus', light: 'light-plus' };

describe('DiffView.scrollToFile', () => {
  beforeEach(() => scrollToIndex.mockClear());

  it('calls scrollToIndex even when the target row is already rendered', () => {
    renderedCount = 3;
    const ref = createRef<DiffViewHandle>();
    render(
      <DiffView
        ref={ref}
        files={[file('a.ts'), file('b.ts'), file('c.ts')]}
        viewType="unified"
        themeType="dark"
        syntaxTheme={SYNTAX_THEME}
      />,
      { wrapper },
    );

    ref.current!.scrollToFile('c.ts');

    expect(scrollToIndex).toHaveBeenCalledTimes(1);
    expect(scrollToIndex).toHaveBeenCalledWith(2, { align: 'start' });
  });

  it('does nothing for an unknown file path', () => {
    renderedCount = 1;
    const ref = createRef<DiffViewHandle>();
    render(
      <DiffView
        ref={ref}
        files={[file('a.ts')]}
        viewType="unified"
        themeType="dark"
        syntaxTheme={SYNTAX_THEME}
      />,
      { wrapper },
    );

    ref.current!.scrollToFile('missing.ts');

    expect(scrollToIndex).not.toHaveBeenCalled();
  });
});
