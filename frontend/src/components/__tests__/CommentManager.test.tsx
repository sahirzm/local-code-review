import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { useEffect, useRef } from 'react';
import { CommentManager } from '../CommentManager.js';
import { ReviewStoreProvider, useReviewStore } from '../../hooks/useReviewStore.js';
import type { Comment, ParsedFileDiff } from '../../shared/types.js';

const diffFiles: ParsedFileDiff[] = [
  {
    oldPath: 'a.ts',
    newPath: 'a.ts',
    status: 'modified',
    additions: 1,
    deletions: 0,
    isBinary: false,
    isLarge: false,
    rawPatch: '',
    hunks: [],
  },
];

/** Seeds a fixed set of comments, then renders the manager against the store. */
function Harness({ seed }: { seed: Omit<Comment, 'id' | 'createdAt' | 'updatedAt'>[] }) {
  const { addComment, getAllComments } = useReviewStore();
  const seeded = useRef(false);
  useEffect(() => {
    if (seeded.current) return;
    seeded.current = true;
    seed.forEach((c) => addComment(c));
  }, [addComment, seed]);
  return (
    <>
      <div data-testid="count">{getAllComments().length}</div>
      <CommentManager open onOpenChange={() => undefined} diffFiles={diffFiles} onJumpToComment={() => undefined} />
    </>
  );
}

describe('CommentManager', () => {
  beforeEach(() => localStorage.clear());

  it('lists comments and filters by status tab', () => {
    render(
      <ReviewStoreProvider>
        <Harness
          seed={[
            { type: 'line', category: 'fix', text: 'open one', filePath: 'a.ts', startLine: 1, side: 'new', status: 'open' },
            { type: 'line', category: 'nit', text: 'resolved one', filePath: 'a.ts', startLine: 2, side: 'new', status: 'resolved' },
          ]}
        />
      </ReviewStoreProvider>,
    );
    expect(screen.getByText('open one')).toBeTruthy();
    expect(screen.getByText('resolved one')).toBeTruthy();

    fireEvent.click(screen.getByRole('tab', { name: /Open/ }));
    expect(screen.getByText('open one')).toBeTruthy();
    expect(screen.queryByText('resolved one')).toBeNull();
  });

  it('surfaces file/overall comments whose file is not in the current diff', () => {
    render(
      <ReviewStoreProvider>
        <Harness
          seed={[
            { type: 'file', category: 'fix', text: 'gone file comment', filePath: 'gone.ts', status: 'open' },
          ]}
        />
      </ReviewStoreProvider>,
    );
    // The comment is invisible in the diff view (file absent) but must appear here.
    expect(screen.getByText('gone file comment')).toBeTruthy();
    expect(screen.getByText('gone.ts')).toBeTruthy();
  });

  it('resolves a comment from the manager', () => {
    render(
      <ReviewStoreProvider>
        <Harness
          seed={[
            { type: 'line', category: 'fix', text: 'toggle me', filePath: 'a.ts', startLine: 1, side: 'new', status: 'open' },
          ]}
        />
      </ReviewStoreProvider>,
    );
    fireEvent.click(screen.getByRole('button', { name: /Resolve/ }));
    expect(screen.getByText('resolved')).toBeTruthy();
  });
});
