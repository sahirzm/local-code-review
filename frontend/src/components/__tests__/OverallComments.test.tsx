import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { OverallComments } from '../OverallComments.js';
import { ReviewStoreProvider } from '../../hooks/useReviewStore.js';

function renderOverall() {
  return render(
    <ReviewStoreProvider>
      <OverallComments />
    </ReviewStoreProvider>,
  );
}

describe('OverallComments', () => {
  beforeEach(() => localStorage.clear());

  it('shows the add button and no form initially', () => {
    renderOverall();
    expect(screen.getByRole('button', { name: '+ Add overall comment' })).toBeTruthy();
    expect(screen.queryByRole('form')).toBeNull();
  });

  it('reveals the form and hides the add button when adding', () => {
    renderOverall();
    fireEvent.click(screen.getByRole('button', { name: '+ Add overall comment' }));
    expect(screen.getByRole('form', { name: 'Add comment' })).toBeTruthy();
    expect(screen.queryByRole('button', { name: '+ Add overall comment' })).toBeNull();
  });

  it('adds an overall comment and renders it as a widget', () => {
    renderOverall();
    fireEvent.click(screen.getByRole('button', { name: '+ Add overall comment' }));
    fireEvent.change(screen.getByLabelText('Comment text'), { target: { value: 'great overall' } });
    fireEvent.click(screen.getByRole('button', { name: 'Submit' }));

    expect(screen.getByText('great overall')).toBeTruthy();
    // Form closes and the add button returns.
    expect(screen.getByRole('button', { name: '+ Add overall comment' })).toBeTruthy();
  });

  it('cancels adding without creating a comment', () => {
    const { container } = renderOverall();
    fireEvent.click(screen.getByRole('button', { name: '+ Add overall comment' }));
    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));

    expect(screen.queryByRole('form')).toBeNull();
    expect(container.querySelectorAll('.comment-widget')).toHaveLength(0);
  });
});
