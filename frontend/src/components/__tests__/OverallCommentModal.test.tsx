import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { OverallCommentModal } from '../OverallCommentModal.js';
import { ReviewStoreProvider, useReviewStore } from '../../hooks/useReviewStore.js';

function Probe(): React.JSX.Element {
  const { comments } = useReviewStore();
  return <output data-testid="probe">{comments.map((c) => `${c.type}:${c.text}`).join('|')}</output>;
}

function renderModal(open = true) {
  const onOpenChange = vi.fn();
  const result = render(
    <ReviewStoreProvider>
      <OverallCommentModal open={open} onOpenChange={onOpenChange} />
      <Probe />
    </ReviewStoreProvider>,
  );
  return { ...result, onOpenChange };
}

describe('OverallCommentModal', () => {
  beforeEach(() => localStorage.clear());

  it('renders nothing while closed', () => {
    renderModal(false);
    expect(screen.queryByRole('form')).toBeNull();
    expect(screen.queryByText('Overall comment')).toBeNull();
  });

  it('opens straight into the form, with no extra click to reveal it', () => {
    renderModal();
    // The old in-column section needed an "+ Add overall comment" press first;
    // the modal is only ever opened in order to write something.
    expect(screen.getByRole('form', { name: 'Add comment' })).toBeTruthy();
  });

  it('adds an overall comment and closes', () => {
    const { onOpenChange } = renderModal();
    fireEvent.change(screen.getByLabelText('Comment text'), { target: { value: 'great overall' } });
    fireEvent.click(screen.getByRole('button', { name: 'Submit' }));

    expect(screen.getByTestId('probe').textContent).toBe('overall:great overall');
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });

  it('cancels without creating a comment', () => {
    const { onOpenChange } = renderModal();
    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));

    expect(screen.getByTestId('probe').textContent).toBe('');
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });
});
