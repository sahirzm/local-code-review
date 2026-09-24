import { useState, useRef, useEffect } from 'react';
import { motion } from 'motion/react';
import { Pencil, Trash2, Check, RotateCcw } from 'lucide-react';
import type { Comment } from '../shared/types.js';
import { CommentForm, renderTextWithCode } from './CommentForm.js';
import { useReviewStore } from '../hooks/useReviewStore.js';

interface CommentWidgetProps {
  comment: Comment;
  isActive?: boolean;
}

function formatTime(iso: string): string {
  const d = new Date(iso);
  return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

export function CommentWidget({ comment, isActive }: CommentWidgetProps): React.JSX.Element {
  const [editing, setEditing] = useState(false);
  const { editComment, deleteComment, setCommentStatus } = useReviewStore();
  const widgetRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (isActive && widgetRef.current) {
      requestAnimationFrame(() => {
        const el = widgetRef.current;
        if (!el) return;
        const rect = el.getBoundingClientRect();
        // Only scroll when the comment is not already fully on screen, so
        // jumping to something you can already see does not move the page.
        if (rect.top < 0 || rect.bottom > window.innerHeight) {
          el.scrollIntoView({ block: 'center' });
        }
      });
    }
  }, [isActive]);

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

  const resolved = comment.status === 'resolved';
  const orphaned = comment.status === 'orphaned';

  return (
    <motion.div
      className={`comment-widget${isActive ? ' comment-widget-active' : ''}${resolved ? ' comment-widget-resolved' : ''}${orphaned ? ' comment-widget-orphaned' : ''}`}
      ref={widgetRef}
      data-comment-id={comment.id}
      initial={{ opacity: 0, y: -4 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.16, ease: [0.16, 1, 0.3, 1] }}
    >
      <div className="comment-widget-header">
        <span className={`category-badge category-badge-${comment.category}`}>
          {comment.category}
        </span>
        {lineLabel && <span className="comment-line-label">{lineLabel}</span>}
        {resolved && <span className="comment-status-badge comment-status-resolved">resolved</span>}
        <span className="comment-time">{formatTime(comment.createdAt)}</span>
        <div className="comment-widget-actions">
          <button
            type="button"
            className="btn-icon"
            onClick={() => setCommentStatus(comment.id, resolved ? 'open' : 'resolved')}
            aria-label={resolved ? 'Reopen comment' : 'Resolve comment'}
            title={resolved ? 'Reopen comment' : 'Resolve comment'}
          >
            {resolved ? <RotateCcw size={14} aria-hidden="true" /> : <Check size={14} aria-hidden="true" />}
          </button>
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
