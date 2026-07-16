import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { CommentForm, renderTextWithCode } from '../CommentForm.js';

describe('renderTextWithCode', () => {
  it('wraps backtick spans in <code> and leaves prose in <span>', () => {
    const { container } = render(<>{renderTextWithCode('use `foo()` here')}</>);
    const code = container.querySelector('code.inline-code');
    expect(code?.textContent).toBe('foo()');
    expect(container.textContent).toBe('use foo() here');
  });

  it('returns a single span when there is no code', () => {
    const { container } = render(<>{renderTextWithCode('plain text')}</>);
    expect(container.querySelector('code')).toBeNull();
    expect(container.textContent).toBe('plain text');
  });
});

describe('CommentForm', () => {
  it('labels itself by mode', () => {
    const { rerender } = render(
      <CommentForm mode="create" onSubmit={vi.fn()} onCancel={vi.fn()} />,
    );
    expect(screen.getByRole('form', { name: 'Add comment' })).toBeTruthy();

    rerender(<CommentForm mode="edit" onSubmit={vi.fn()} onCancel={vi.fn()} />);
    expect(screen.getByRole('form', { name: 'Edit comment' })).toBeTruthy();
  });

  it('disables submit until non-whitespace text is entered', () => {
    render(<CommentForm mode="create" onSubmit={vi.fn()} onCancel={vi.fn()} />);
    const submit = screen.getByRole('button', { name: 'Submit' }) as HTMLButtonElement;
    expect(submit.disabled).toBe(true);

    fireEvent.change(screen.getByLabelText('Comment text'), { target: { value: '   ' } });
    expect(submit.disabled).toBe(true);

    fireEvent.change(screen.getByLabelText('Comment text'), { target: { value: 'real' } });
    expect(submit.disabled).toBe(false);
  });

  it('submits trimmed text with the selected category', () => {
    const onSubmit = vi.fn();
    render(<CommentForm mode="create" onSubmit={onSubmit} onCancel={vi.fn()} />);
    fireEvent.click(screen.getByRole('button', { name: 'nit' }));
    fireEvent.change(screen.getByLabelText('Comment text'), { target: { value: '  needs a test  ' } });
    fireEvent.click(screen.getByRole('button', { name: 'Submit' }));
    expect(onSubmit).toHaveBeenCalledWith('needs a test', 'nit');
  });

  it('seeds initial text and category in edit mode and shows Save', () => {
    render(
      <CommentForm
        mode="edit"
        initialText="existing"
        initialCategory="question"
        onSubmit={vi.fn()}
        onCancel={vi.fn()}
      />,
    );
    expect((screen.getByLabelText('Comment text') as HTMLTextAreaElement).value).toBe('existing');
    expect(screen.getByRole('button', { name: 'question' }).getAttribute('aria-pressed')).toBe('true');
    expect(screen.getByRole('button', { name: 'Save' })).toBeTruthy();
  });

  it('submits on Ctrl+Enter and cancels on Escape', () => {
    const onSubmit = vi.fn();
    const onCancel = vi.fn();
    render(<CommentForm mode="create" onSubmit={onSubmit} onCancel={onCancel} />);
    const textarea = screen.getByLabelText('Comment text');

    fireEvent.change(textarea, { target: { value: 'ship it' } });
    fireEvent.keyDown(textarea, { key: 'Enter', ctrlKey: true });
    expect(onSubmit).toHaveBeenCalledWith('ship it', 'fix');

    fireEvent.keyDown(textarea, { key: 'Escape' });
    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it('does not submit empty text on Ctrl+Enter', () => {
    const onSubmit = vi.fn();
    render(<CommentForm mode="create" onSubmit={onSubmit} onCancel={vi.fn()} />);
    fireEvent.keyDown(screen.getByLabelText('Comment text'), { key: 'Enter', ctrlKey: true });
    expect(onSubmit).not.toHaveBeenCalled();
  });

  it('enforces the 2000-character cap and warns near the limit', () => {
    render(<CommentForm mode="create" onSubmit={vi.fn()} onCancel={vi.fn()} />);
    const textarea = screen.getByLabelText('Comment text') as HTMLTextAreaElement;

    fireEvent.change(textarea, { target: { value: 'z'.repeat(2500) } });
    expect(textarea.value.length).toBe(2000);
    expect(screen.getByText('0 characters remaining').className).toContain('warn');
  });

  it('cancels via the Cancel button', () => {
    const onCancel = vi.fn();
    render(<CommentForm mode="create" onSubmit={vi.fn()} onCancel={onCancel} />);
    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(onCancel).toHaveBeenCalledTimes(1);
  });
});
