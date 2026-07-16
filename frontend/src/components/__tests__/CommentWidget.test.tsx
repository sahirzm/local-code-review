import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { useEffect, useRef } from 'react';
import { CommentWidget } from '../CommentWidget.js';
import { ReviewStoreProvider, useReviewStore } from '../../hooks/useReviewStore.js';
import type { Comment } from '../../shared/types.js';

function comment(overrides: Partial<Comment> = {}): Comment {
  return {
    id: 'c1',
    type: 'line',
    category: 'fix',
    text: 'Looks off',
    filePath: 'a.ts',
    startLine: 5,
    side: 'new',
    createdAt: '2026-01-01T10:30:00.000Z',
    updatedAt: '2026-01-01T10:30:00.000Z',
    ...overrides,
  };
}

/**
 * Seeds one comment into the store on mount, then renders its widget plus a
 * readout of the store — so the widget and the store observer share one
 * provider (the store is per-provider in-memory state).
 */
function StoreHarness({ text = 'first' }: { text?: string }) {
  const { addComment, comments } = useReviewStore();
  const seeded = useRef(false);
  useEffect(() => {
    if (seeded.current) return;
    seeded.current = true;
    addComment({ type: 'line', category: 'fix', text, filePath: 'a.ts', startLine: 1, side: 'new' });
  }, [addComment, text]);

  return (
    <>
      <div data-testid="count">{comments.length}</div>
      {comments.map((c) => (
        <div key={c.id} data-testid="stored-text">
          {c.text}
          <CommentWidget comment={c} />
        </div>
      ))}
    </>
  );
}

describe('CommentWidget', () => {
  beforeEach(() => localStorage.clear());

  it('renders category, line label and body text', () => {
    render(
      <ReviewStoreProvider>
        <CommentWidget comment={comment()} />
      </ReviewStoreProvider>,
    );
    expect(screen.getByText('fix')).toBeTruthy();
    expect(screen.getByText('L5')).toBeTruthy();
    expect(screen.getByText('Looks off')).toBeTruthy();
  });

  it('renders a range label for a multi-line range comment', () => {
    render(
      <ReviewStoreProvider>
        <CommentWidget comment={comment({ type: 'range', startLine: 5, endLine: 9 })} />
      </ReviewStoreProvider>,
    );
    expect(screen.getByText('L5-9')).toBeTruthy();
  });

  it('labels file-level comments as "File"', () => {
    render(
      <ReviewStoreProvider>
        <CommentWidget comment={comment({ type: 'file', startLine: undefined })} />
      </ReviewStoreProvider>,
    );
    expect(screen.getByText('File')).toBeTruthy();
  });

  it('renders backtick text as inline code', () => {
    const { container } = render(
      <ReviewStoreProvider>
        <CommentWidget comment={comment({ text: 'call `run()`' })} />
      </ReviewStoreProvider>,
    );
    expect(container.querySelector('code.inline-code')?.textContent).toBe('run()');
  });

  it('marks the active widget with a modifier class', () => {
    const { container } = render(
      <ReviewStoreProvider>
        <CommentWidget comment={comment()} isActive />
      </ReviewStoreProvider>,
    );
    expect(container.querySelector('.comment-widget-active')).not.toBeNull();
  });

  it('enters edit mode and saves changes through the store', () => {
    render(
      <ReviewStoreProvider>
        <StoreHarness text="first" />
      </ReviewStoreProvider>,
    );
    fireEvent.click(screen.getByLabelText('Edit comment'));
    fireEvent.change(screen.getByLabelText('Comment text'), { target: { value: 'edited body' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save' }));

    expect(screen.getByTestId('stored-text').textContent).toContain('edited body');
  });

  it('deletes through the store', () => {
    render(
      <ReviewStoreProvider>
        <StoreHarness text="gone soon" />
      </ReviewStoreProvider>,
    );
    expect(screen.getByTestId('count').textContent).toBe('1');
    fireEvent.click(screen.getByLabelText('Delete comment'));
    expect(screen.getByTestId('count').textContent).toBe('0');
  });
});
