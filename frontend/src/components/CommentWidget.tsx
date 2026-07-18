import { useState, useRef, useEffect } from 'react';
import { motion } from 'motion/react';
import { Pencil, Trash2 } from 'lucide-react';
import type { Comment } from '../../../shared/types.js';
import { CommentForm, renderTextWithCode } from './CommentForm.js';
import { useReviewStore } from '../hooks/useReviewStore.js';

interface CommentWidgetProps {
  comment: Comment;
  isActive?: boolean;
  scrollDirection?: 'forward' | 'backward' | null;
}

function formatTime(iso: string): string {
  const d = new Date(iso);
  return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

export function CommentWidget({ comment, isActive, scrollDirection }: CommentWidgetProps): React.JSX.Element {
  const [editing, setEditing] = useState(false);
  const { editComment, deleteComment } = useReviewStore();
  const widgetRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (isActive && widgetRef.current) {
      requestAnimationFrame(() => {
        const el = widgetRef.current;
        if (!el) return;
        const rect = el.getBoundingClientRect();
        const OFFSET = 60;
        const BACK_OFFSET = 200; // ~10 lines of context above
        if (scrollDirection === 'forward' && rect.bottom > window.innerHeight - OFFSET) {
          // Scrolling forward: ensure bottom of comment is visible with offset
          el.scrollIntoView({ block: 'end' });
          el.closest('.diff-view-scroll')?.scrollBy(0, OFFSET);
        } else if (scrollDirection === 'backward' && rect.top < BACK_OFFSET) {
          // Scrolling backward: show ~10 lines of context above the comment
          el.scrollIntoView({ block: 'start' });
          el.closest('.diff-view-scroll')?.scrollBy(0, -BACK_OFFSET);
        } else if (rect.bottom > window.innerHeight || rect.top < 0) {
          // Fully off-screen: just bring into view
          el.scrollIntoView({ block: 'nearest' });
        }
      });
    }
  }, [isActive, scrollDirection]);

  if (editing) {
    return (
      <div className="comment-widget">
        <CommentForm
          mode="edit"
          initialText={comment.text}
          initialCategory={comment.category}
          onSubmit={(text, category) => {
            editComment(comment.id, { text, category });
            setEditing(false);
          }}
          onCancel={() => setEditing(false)}
        />
      </div>
    );
  }

  const lineLabel = comment.type === 'range' && comment.startLine != null && comment.endLine != null && comment.endLine > comment.startLine
    ? `L${comment.startLine}-${comment.endLine}`
    : comment.type === 'line' && comment.startLine != null
      ? `L${comment.startLine}`
      : comment.type === 'file'
        ? 'File'
        : null;

  return (
    <motion.div
      className={`comment-widget${isActive ? ' comment-widget-active' : ''}`}
      ref={widgetRef}
      initial={{ opacity: 0, y: -4 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.16, ease: [0.16, 1, 0.3, 1] }}
    >
      <div className="comment-widget-header">
        <span className={`category-badge category-badge-${comment.category}`}>
          {comment.category}
        </span>
        {lineLabel && <span className="comment-line-label">{lineLabel}</span>}
        <span className="comment-time">{formatTime(comment.createdAt)}</span>
        <div className="comment-widget-actions">
          <button type="button" className="btn-icon" onClick={() => setEditing(true)} aria-label="Edit comment">
            <Pencil size={14} aria-hidden="true" />
          </button>
          <button type="button" className="btn-icon" onClick={() => deleteComment(comment.id)} aria-label="Delete comment">
            <Trash2 size={14} aria-hidden="true" />
          </button>
        </div>
      </div>
      <div className="comment-widget-text">{renderTextWithCode(comment.text)}</div>
    </motion.div>
  );
}
