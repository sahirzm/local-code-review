import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { FileDiff } from '../FileDiff.js';
import { ReviewStoreProvider } from '../../hooks/useReviewStore.js';
import type { ParsedFileDiff, Hunk, Change } from '../../shared/types.js';
import type { ReactNode } from 'react';

function wrapper({ children }: { children: ReactNode }) {
  return <ReviewStoreProvider>{children}</ReviewStoreProvider>;
}

function change(type: Change['type'], content: string, old?: number, new_?: number): Change {
  return { type, content, oldLineNumber: old, newLineNumber: new_ };
}

function hunk(content: string, changes: Change[], overrides: Partial<Hunk> = {}): Hunk {
  return { oldStart: 1, oldLines: changes.length, newStart: 1, newLines: changes.length, content, changes, ...overrides };
}

function fileDiff(hunks: Hunk[]): ParsedFileDiff {
  return {
    oldPath: 'a.ts',
    newPath: 'a.ts',
    hunks,
    status: 'modified',
    additions: 1,
    deletions: 1,
    isBinary: false,
    isLarge: false,
  };
}

describe('FileDiff hunk separators', () => {
  it('renders no separator for a single hunk', () => {
    const file = fileDiff([
      hunk('@@ -1,2 +1,2 @@', [change('normal', ' a', 1, 1), change('insert', '+b', undefined, 2)]),
    ]);
    render(<FileDiff file={file} viewType="unified" />, { wrapper });
    expect(screen.queryAllByText(/@@ .* @@/)).toHaveLength(0);
  });

  it('renders a separator between two hunks with non-contiguous line ranges', () => {
    const file = fileDiff([
      hunk('@@ -1,3 +1,3 @@', [
        change('normal', ' a', 1, 1),
        change('insert', '+b', undefined, 2),
        change('normal', ' c', 3, 3),
      ]),
      hunk('@@ -40,3 +41,3 @@', [
        change('normal', ' x', 40, 41),
        change('delete', '-y', 41),
        change('normal', ' z', 42, 42),
      ], { oldStart: 40, newStart: 41 }),
    ]);
    const { container } = render(<FileDiff file={file} viewType="unified" />, { wrapper });
    const separators = container.querySelectorAll('.hunk-separator');
    expect(separators).toHaveLength(1);
    expect(separators[0].textContent).toBe('@@ -40,3 +41,3 @@');
  });

  it('renders N-1 separators for N hunks', () => {
    const file = fileDiff([
      hunk('@@ -1,1 +1,1 @@', [change('insert', '+a', undefined, 1)]),
      hunk('@@ -20,1 +21,1 @@', [change('insert', '+b', undefined, 21)], { oldStart: 20, newStart: 21 }),
      hunk('@@ -40,1 +41,1 @@', [change('insert', '+c', undefined, 41)], { oldStart: 40, newStart: 41 }),
    ]);
    const { container } = render(<FileDiff file={file} viewType="unified" />, { wrapper });
    const separators = container.querySelectorAll('.hunk-separator');
    expect(separators).toHaveLength(2);
  });
});
