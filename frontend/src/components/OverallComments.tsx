import { useState } from 'react';
import type { Comment } from '../../../shared/types.js';
import { useReviewStore } from '../hooks/useReviewStore.js';
import { CommentForm } from './CommentForm.js';
import { CommentWidget } from './CommentWidget.js';

interface OverallCommentsProps {
  activeCommentId?: string | null;
  scrollDirection?: 'forward' | 'backward' | null;
}

export function OverallComments({ activeCommentId, scrollDirection }: OverallCommentsProps): React.JSX.Element {
  const [showForm, setShowForm] = useState(false);
  const { comments, addComment } = useReviewStore();

  const overallComments = comments.filter((c) => c.type === 'overall');

  return (
    <section className="overall-comments" aria-label="Overall comments">
      <div className="overall-comments-header">
        <h2>Overall Comments</h2>
        {!showForm && (
          <button type="button" className="btn btn-add" onClick={() => setShowForm(true)}>
            + Add overall comment
          </button>
        )}
      </div>
      {showForm && (
        <CommentForm
          mode="create"
          onSubmit={(text: string, category: Comment['category']) => {
            addComment({ type: 'overall', category, text });
            setShowForm(false);
          }}
          onCancel={() => setShowForm(false)}
        />
      )}
      {overallComments.map((c) => (
        <CommentWidget key={c.id} comment={c} isActive={c.id === activeCommentId} scrollDirection={c.id === activeCommentId ? scrollDirection : null} />
      ))}
    </section>
  );
}
