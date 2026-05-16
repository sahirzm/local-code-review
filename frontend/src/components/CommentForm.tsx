import { useState, useCallback, useRef, useEffect, type KeyboardEvent, type ReactNode } from 'react';
import type { Comment } from '../../../shared/types.js';

const MAX_CHARS = 2000;
const CATEGORIES: Comment['category'][] = ['fix', 'question', 'suggestion', 'nit'];

interface CommentFormProps {
  onSubmit: (text: string, category: Comment['category']) => void;
  onCancel: () => void;
  initialText?: string;
  initialCategory?: Comment['category'];
  mode: 'create' | 'edit';
}

function renderTextWithCode(text: string): ReactNode[] {
  const parts = text.split(/(`[^`]+`)/g);
  return parts.map((part, i) =>
    part.startsWith('`') && part.endsWith('`') ? (
      <code key={i} className="inline-code">{part.slice(1, -1)}</code>
    ) : (
      <span key={i}>{part}</span>
    ),
  );
}

export { renderTextWithCode };

export function CommentForm({
  onSubmit,
  onCancel,
  initialText = '',
  initialCategory = 'fix',
  mode,
}: CommentFormProps): React.JSX.Element {
  const [text, setText] = useState(initialText);
  const [category, setCategory] = useState<Comment['category']>(initialCategory);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    textareaRef.current?.focus();
  }, []);

  // Auto-grow textarea to fit content
  useEffect(() => {
    const el = textareaRef.current;
    if (el) {
      el.style.height = 'auto';
      el.style.height = `${Math.max(80, el.scrollHeight)}px`;
    }
  }, [text]);

  const handleSubmit = useCallback(() => {
    const trimmed = text.trim();
    if (trimmed.length === 0) return;
    onSubmit(trimmed, category);
  }, [text, category, onSubmit]);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        handleSubmit();
      } else if (e.key === 'Escape') {
        e.preventDefault();
        onCancel();
      }
    },
    [handleSubmit, onCancel],
  );

  const remaining = MAX_CHARS - text.length;

  return (
    <div className="comment-form" role="form" aria-label={mode === 'create' ? 'Add comment' : 'Edit comment'}>
      <div className="comment-form-categories" role="group" aria-label="Comment category">
        {CATEGORIES.map((cat) => (
          <button
            key={cat}
            type="button"
            className={`category-pill category-pill-${cat}${category === cat ? ' active' : ''}`}
            onClick={() => setCategory(cat)}
            aria-pressed={category === cat}
          >
            {cat}
          </button>
        ))}
      </div>
      <textarea
        ref={textareaRef}
        className="comment-form-textarea"
        value={text}
        onChange={(e) => setText(e.target.value.slice(0, MAX_CHARS))}
        onKeyDown={handleKeyDown}
        placeholder="Write a comment… (Ctrl+Enter to submit, Shift+click gutter for range)"
        maxLength={MAX_CHARS}
        rows={3}
        aria-label="Comment text"
      />
      <div className="comment-form-footer">
        <span className={`char-counter${remaining < 100 ? ' warn' : ''}`}>
          {remaining} characters remaining
        </span>
        <div className="comment-form-actions">
          <button type="button" className="btn btn-cancel" onClick={onCancel}>
            Cancel
          </button>
          <button
            type="button"
            className="btn btn-submit"
            onClick={handleSubmit}
            disabled={text.trim().length === 0}
          >
            {mode === 'create' ? 'Submit' : 'Save'}
          </button>
        </div>
      </div>
    </div>
  );
}
