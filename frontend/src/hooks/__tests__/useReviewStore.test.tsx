import { describe, it, expect, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { ReviewStoreProvider, useReviewStore } from '../useReviewStore.js';
import type { Comment } from '../../shared/types.js';
import type { ReactNode } from 'react';

function wrapper({ children }: { children: ReactNode }) {
  return <ReviewStoreProvider>{children}</ReviewStoreProvider>;
}

function commentInput(overrides: Partial<Omit<Comment, 'id' | 'createdAt' | 'updatedAt'>> = {}) {
  return {
    type: 'line' as const,
    category: 'fix' as const,
    text: 'Test comment',
    filePath: 'test.ts',
    startLine: 1,
    side: 'new' as const,
    ...overrides,
  };
}

describe('useReviewStore', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('starts with empty state', () => {
    const { result } = renderHook(() => useReviewStore(), { wrapper });
    expect(result.current.comments).toEqual([]);
    expect(result.current.viewMode).toBe('split');
    expect(result.current.reviewedFiles).toEqual([]);
  });

  it('adds a comment', () => {
    const { result } = renderHook(() => useReviewStore(), { wrapper });

    act(() => {
      result.current.addComment(commentInput({ text: 'Hello' }));
    });

    expect(result.current.comments).toHaveLength(1);
    expect(result.current.comments[0].text).toBe('Hello');
    expect(result.current.comments[0].id).toBeTruthy();
    expect(result.current.comments[0].createdAt).toBeTruthy();
  });

  it('edits a comment', () => {
    const { result } = renderHook(() => useReviewStore(), { wrapper });

    act(() => {
      result.current.addComment(commentInput({ text: 'Original' }));
    });

    const id = result.current.comments[0].id;

    act(() => {
      result.current.editComment(id, { text: 'Updated', category: 'nit' });
    });

    expect(result.current.comments[0].text).toBe('Updated');
    expect(result.current.comments[0].category).toBe('nit');
  });

  it('deletes a comment', () => {
    const { result } = renderHook(() => useReviewStore(), { wrapper });

    act(() => {
      result.current.addComment(commentInput());
    });

    const id = result.current.comments[0].id;

    act(() => {
      result.current.deleteComment(id);
    });

    expect(result.current.comments).toHaveLength(0);
  });

  it('toggles view mode', () => {
    const { result } = renderHook(() => useReviewStore(), { wrapper });

    act(() => {
      result.current.setViewMode('unified');
    });

    expect(result.current.viewMode).toBe('unified');

    act(() => {
      result.current.setViewMode('split');
    });

    expect(result.current.viewMode).toBe('split');
  });

  it('marks and unmarks files as reviewed', () => {
    const { result } = renderHook(() => useReviewStore(), { wrapper });

    act(() => {
      result.current.markFileReviewed('a.ts');
    });

    expect(result.current.isFileReviewed('a.ts')).toBe(true);
    expect(result.current.isFileReviewed('b.ts')).toBe(false);

    act(() => {
      result.current.unmarkFileReviewed('a.ts');
    });

    expect(result.current.isFileReviewed('a.ts')).toBe(false);
  });

  it('does not duplicate reviewed files', () => {
    const { result } = renderHook(() => useReviewStore(), { wrapper });

    act(() => {
      result.current.markFileReviewed('a.ts');
      result.current.markFileReviewed('a.ts');
    });

    expect(result.current.reviewedFiles).toEqual(['a.ts']);
  });

  it('filters comments by file', () => {
    const { result } = renderHook(() => useReviewStore(), { wrapper });

    act(() => {
      result.current.addComment(commentInput({ filePath: 'a.ts', text: 'A' }));
      result.current.addComment(commentInput({ filePath: 'b.ts', text: 'B' }));
    });

    const aComments = result.current.getCommentsForFile('a.ts');
    expect(aComments).toHaveLength(1);
    expect(aComments[0].text).toBe('A');
  });

  it('filters comments by line and side', () => {
    const { result } = renderHook(() => useReviewStore(), { wrapper });

    act(() => {
      result.current.addComment(
        commentInput({ filePath: 'a.ts', startLine: 5, endLine: 10, side: 'new', text: 'Range' }),
      );
      result.current.addComment(
        commentInput({ filePath: 'a.ts', startLine: 20, side: 'new', text: 'Other line' }),
      );
    });

    const line7 = result.current.getCommentsForLine('a.ts', 7, 'new');
    expect(line7).toHaveLength(1);
    expect(line7[0].text).toBe('Range');

    const line20 = result.current.getCommentsForLine('a.ts', 20, 'new');
    expect(line20).toHaveLength(1);
    expect(line20[0].text).toBe('Other line');

    const line99 = result.current.getCommentsForLine('a.ts', 99, 'new');
    expect(line99).toHaveLength(0);
  });

  it('discards all state', () => {
    const { result } = renderHook(() => useReviewStore(), { wrapper });

    act(() => {
      result.current.addComment(commentInput());
      result.current.markFileReviewed('a.ts');
      result.current.setViewMode('unified');
    });

    act(() => {
      result.current.discardReview();
    });

    expect(result.current.comments).toEqual([]);
    expect(result.current.reviewedFiles).toEqual([]);
    expect(result.current.viewMode).toBe('split');
  });

  it('getAllComments returns all comments', () => {
    const { result } = renderHook(() => useReviewStore(), { wrapper });

    act(() => {
      result.current.addComment(commentInput({ text: 'One' }));
      result.current.addComment(commentInput({ text: 'Two' }));
    });

    expect(result.current.getAllComments()).toHaveLength(2);
  });
});
