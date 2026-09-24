import { Modal } from './ui/Modal.js';
import { CommentForm } from './CommentForm.js';
import { useReviewStore } from '../hooks/useReviewStore.js';
import type { Comment } from '../shared/types.js';

interface OverallCommentModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

/**
 * Composer for review-wide comments.
 *
 * This used to be a section pinned above the diff, which cost vertical space on
 * every file whether or not you were writing anything. Existing overall
 * comments are listed in the comment manager, so this only needs to take one.
 */
export function OverallCommentModal({ open, onOpenChange }: OverallCommentModalProps): React.JSX.Element {
  const { comments, addComment } = useReviewStore();
  const existing = comments.filter((c) => c.type === 'overall').length;

  return (
    <Modal
      open={open}
      onOpenChange={onOpenChange}
      ariaLabel="Add overall comment"
      dialogClassName="modal-dialog overall-comment-dialog"
    >
      <div className="settings-header">
        <h2>
          Overall comment
          {existing > 0 && <span className="overall-comments-count"> ({existing} so far)</span>}
        </h2>
      </div>
      <p className="overall-comment-hint">
        Applies to the whole review rather than a file or a line.
      </p>
      <CommentForm
        mode="create"
        onSubmit={(text: string, category: Comment['category']) => {
          addComment({ type: 'overall', category, text });
          onOpenChange(false);
        }}
        onCancel={() => onOpenChange(false)}
      />
    </Modal>
  );
}
