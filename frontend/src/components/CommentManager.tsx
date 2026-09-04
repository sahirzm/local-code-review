import { useMemo, useState } from 'react';
import { X, Check, RotateCcw, Trash2, CornerUpRight, Link2 } from 'lucide-react';
import { Modal } from './ui/Modal.js';
import { useReviewStore } from '../hooks/useReviewStore.js';
import type { Comment, CommentStatus, ParsedFileDiff } from '../shared/types.js';

type StatusTab = 'all' | 'open' | 'resolved' | 'orphaned';

const STATUS_TABS: { value: StatusTab; label: string }[] = [
  { value: 'all', label: 'All' },
  { value: 'open', label: 'Open' },
  { value: 'resolved', label: 'Resolved' },
  { value: 'orphaned', label: 'Orphaned' },
];

const ORPHAN_REASON_LABEL: Record<string, string> = {
  snippet_mismatch: 'line content changed',
  file_missing: 'file no longer in diff',
  line_out_of_range: 'line no longer exists',
};

/** Short human label for where a comment is anchored. */
function locationLabel(c: Comment): string {
  if (c.type === 'overall') return 'overall';
  if (!c.filePath) return '(no file)';
  if (c.type === 'file') return c.filePath;
  if (c.startLine != null) {
    const end = c.endLine != null && c.endLine !== c.startLine ? `-${c.endLine}` : '';
    return `${c.filePath}:${c.startLine}${end}`;
  }
  return c.filePath;
}

function effectiveStatus(c: Comment): CommentStatus {
  return c.status ?? 'open';
}

interface CommentManagerProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  diffFiles: ParsedFileDiff[];
  /** Jump to a comment in the diff view (switches file + scrolls). */
  onJumpToComment: (comment: Comment) => void;
}

/**
 * Workspace-wide comment list. Surfaces every comment — including ones that are
 * otherwise invisible in the diff view (orphaned pins whose line drifted, and
 * file/overall comments whose file left the current diff). Filterable by
 * status; per comment you can resolve/reopen, delete, re-pin an orphan, or
 * jump to its line in the diff.
 */
export function CommentManager({ open, onOpenChange, diffFiles, onJumpToComment }: CommentManagerProps): React.JSX.Element {
  const { getAllComments, setCommentStatus, deleteComment, repinComment } = useReviewStore();
  const [tab, setTab] = useState<StatusTab>('all');
  const comments = getAllComments();

  const counts = useMemo(() => {
    const c = { all: comments.length, open: 0, resolved: 0, orphaned: 0 };
    for (const cm of comments) {
      const s = effectiveStatus(cm);
      if (s === 'open') c.open++;
      else if (s === 'resolved') c.resolved++;
      else if (s === 'orphaned') c.orphaned++;
    }
    return c;
  }, [comments]);

  const visible = useMemo(() => {
    const filtered = tab === 'all' ? comments : comments.filter((c) => effectiveStatus(c) === tab);
    return [...filtered].sort((a, b) => {
      const aFile = a.filePath ?? '';
      const bFile = b.filePath ?? '';
      if (aFile !== bFile) return aFile.localeCompare(bFile);
      return (a.startLine ?? 0) - (b.startLine ?? 0);
    });
  }, [comments, tab]);

  const fileIsPresent = (c: Comment): boolean =>
    c.filePath == null || diffFiles.some((f) => f.newPath === c.filePath || f.oldPath === c.filePath);

  return (
    <Modal open={open} onOpenChange={onOpenChange} ariaLabel="Comment manager" dialogClassName="comment-manager-dialog">
      <div className="settings-header">
        <h2>Comments</h2>
        <button type="button" className="btn-icon" onClick={() => onOpenChange(false)} aria-label="Close">
          <X size={16} aria-hidden="true" />
        </button>
      </div>
      <div className="comment-manager-tabs" role="tablist" aria-label="Filter comments by status">
        {STATUS_TABS.map((t) => (
          <button
            key={t.value}
            type="button"
            role="tab"
            aria-selected={tab === t.value}
            className={`comment-manager-tab${tab === t.value ? ' active' : ''}`}
            onClick={() => setTab(t.value)}
          >
            {t.label} ({counts[t.value]})
          </button>
        ))}
      </div>
      <div className="comment-manager-body">
        {visible.length === 0 ? (
          <p className="comment-manager-empty">No comments.</p>
        ) : (
          <ul className="comment-manager-list">
            {visible.map((c) => {
              const status = effectiveStatus(c);
              const present = fileIsPresent(c);
              return (
                <li key={c.id} className={`comment-manager-item status-${status}`} data-comment-id={c.id}>
                  <div className="comment-manager-item-head">
                    <span className={`category-badge category-badge-${c.category}`}>{c.category}</span>
                    <span className="comment-manager-loc">{locationLabel(c)}</span>
                    {status === 'resolved' && <span className="comment-status-badge comment-status-resolved">resolved</span>}
                    {status === 'orphaned' && (
                      <span className="comment-status-badge comment-status-orphaned">
                        orphaned{c.orphanReason ? ` · ${ORPHAN_REASON_LABEL[c.orphanReason] ?? c.orphanReason}` : ''}
                      </span>
                    )}
                  </div>
                  <div className="comment-manager-text">{c.text}</div>
                  <div className="comment-manager-actions">
                    {status !== 'orphaned' && present && (c.type === 'line' || c.type === 'range' || c.type === 'file' || c.type === 'overall') && (
                      <button
                        type="button"
                        className="link-btn"
                        onClick={() => { onJumpToComment(c); onOpenChange(false); }}
                      >
                        <CornerUpRight size={13} aria-hidden="true" /> Jump
                      </button>
                    )}
                    {status === 'orphaned' && (
                      <button
                        type="button"
                        className="link-btn"
                        onClick={() => { repinComment(c.id, diffFiles); }}
                        title="Try to re-anchor this comment to a matching line in the current diff"
                      >
                        <Link2 size={13} aria-hidden="true" /> Re-pin
                      </button>
                    )}
                    {status !== 'orphaned' && (
                      <button
                        type="button"
                        className="link-btn"
                        onClick={() => setCommentStatus(c.id, status === 'resolved' ? 'open' : 'resolved')}
                      >
                        {status === 'resolved'
                          ? <><RotateCcw size={13} aria-hidden="true" /> Reopen</>
                          : <><Check size={13} aria-hidden="true" /> Resolve</>}
                      </button>
                    )}
                    <button type="button" className="link-btn link-danger" onClick={() => deleteComment(c.id)}>
                      <Trash2 size={13} aria-hidden="true" /> Delete
                    </button>
                  </div>
                </li>
              );
            })}
          </ul>
        )}
      </div>
    </Modal>
  );
}
