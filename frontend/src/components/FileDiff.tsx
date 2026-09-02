import { useState, useEffect, useMemo, useCallback } from 'react';
import { ChevronRight, ChevronDown, Check, Circle, MessageSquare, MessageSquarePlus } from 'lucide-react';
import { FileDiff as PierreFileDiff } from '@pierre/diffs/react';
import type { DiffLineAnnotation, SelectedLineRange } from '@pierre/diffs';
import type { ParsedFileDiff, Comment } from '../../../shared/types.js';
import { useReviewStore } from '../hooks/useReviewStore.js';
import { CommentForm } from './CommentForm.js';
import { CommentWidget } from './CommentWidget.js';
import { useIsWorkerPoolReadyOrDisabled } from './diff/workerPool.js';
import {
  parsePatch,
  commentsToAnnotations,
  selectionToAnchor,
  type DiffAnnotationMetadata,
} from './diff/adapter.js';
import { computePin } from '../utils/pin.js';
import type { ShikiThemePair } from './diff/shikiTheme.js';

export type PierreViewType = 'split' | 'unified';

interface FileDiffProps {
  file: ParsedFileDiff;
  viewType: PierreViewType;
  themeType: 'dark' | 'light';
  syntaxTheme: ShikiThemePair;
  activeCommentId?: string | null;
  scrollDirection?: 'forward' | 'backward' | null;
}

interface ActiveForm {
  line: number;
  endLine: number;
  side: 'old' | 'new';
}

function annotationKey(side: 'old' | 'new', line: number): string {
  return `${side}:${line}`;
}

export function FileDiff({
  file,
  viewType,
  themeType,
  syntaxTheme,
  activeCommentId,
  scrollDirection,
}: FileDiffProps): React.JSX.Element {
  const isLargeDefault = file.isLarge;
  const { addComment, getCommentsForFile, isFileReviewed, markFileReviewed, unmarkFileReviewed } = useReviewStore();
  const filePath = file.newPath || file.oldPath;
  const reviewed = isFileReviewed(filePath);
  const [collapsed, setCollapsed] = useState(isLargeDefault || reviewed);
  const [activeForm, setActiveForm] = useState<ActiveForm | null>(null);
  const [showFileForm, setShowFileForm] = useState(false);
  const poolReady = useIsWorkerPoolReadyOrDisabled();

  const fileComments = getCommentsForFile(filePath);
  const hasActiveComment = activeCommentId != null && fileComments.some((c) => c.id === activeCommentId);
  useEffect(() => {
    if (hasActiveComment && collapsed) setCollapsed(false);
  }, [hasActiveComment, collapsed]);

  const fileDiff = useMemo(() => parsePatch(file), [file]);

  // Line/range comments become pierre line annotations; the open comment form
  // is injected as a synthetic annotation on its anchor line so it renders
  // inline exactly where a persisted comment would.
  const annotations = useMemo<DiffLineAnnotation<DiffAnnotationMetadata>[]>(() => {
    const base = commentsToAnnotations(fileComments);
    if (activeForm) {
      const key = annotationKey(activeForm.side, activeForm.endLine);
      const existing = base.find((a) => annotationKey(a.metadata.side, a.metadata.lineNumber) === key);
      if (!existing) {
        base.push({
          side: activeForm.side === 'old' ? 'deletions' : 'additions',
          lineNumber: activeForm.endLine,
          metadata: { comments: [], side: activeForm.side, lineNumber: activeForm.endLine },
        });
      }
    }
    return base;
  }, [fileComments, activeForm]);

  const commentCount = fileComments.length;
  const fileCommentsOfType = fileComments.filter((c) => c.type === 'file');

  const handleFileComment = useCallback(
    (text: string, category: Comment['category']) => {
      addComment({ type: 'file', category, text, filePath });
      setShowFileForm(false);
    },
    [addComment, filePath],
  );

  const handleToggleReviewed = useCallback(() => {
    if (reviewed) {
      unmarkFileReviewed(filePath);
    } else {
      markFileReviewed(filePath);
      setCollapsed(true);
    }
  }, [reviewed, filePath, markFileReviewed, unmarkFileReviewed]);

  const onLineSelectionEnd = useCallback((range: SelectedLineRange | null) => {
    if (!range) {
      setActiveForm(null);
      return;
    }
    setActiveForm(selectionToAnchor(range));
  }, []);

  // The gutter "+" button emits a single-line range; open the comment form on
  // that line (same anchor path as a drag selection).
  const onGutterUtilityClick = useCallback((range: SelectedLineRange) => {
    setActiveForm(selectionToAnchor(range));
  }, []);

  const renderAnnotation = useCallback(
    (annotation: DiffLineAnnotation<DiffAnnotationMetadata>) => {
      const { comments, side, lineNumber } = annotation.metadata;
      const isFormLine = activeForm != null && activeForm.side === side && activeForm.endLine === lineNumber;
      return (
        <div className="comment-widget-container">
          {comments.map((c) => (
            <CommentWidget
              key={c.id}
              comment={c}
              isActive={c.id === activeCommentId}
              scrollDirection={c.id === activeCommentId ? scrollDirection : null}
            />
          ))}
          {isFormLine && (
            <CommentForm
              mode="create"
              onSubmit={(text: string, category: Comment['category']) => {
                const isRange = activeForm.endLine !== activeForm.line;
                const pin = computePin([file], filePath, activeForm.side, activeForm.line, activeForm.endLine);
                addComment({
                  type: isRange ? 'range' : 'line',
                  category,
                  text,
                  filePath,
                  startLine: activeForm.line,
                  endLine: activeForm.endLine,
                  side: activeForm.side,
                  status: 'open',
                  ...(pin ?? {}),
                });
                setActiveForm(null);
              }}
              onCancel={() => setActiveForm(null)}
            />
          )}
        </div>
      );
    },
    [activeForm, activeCommentId, scrollDirection, addComment, filePath],
  );

  if (file.isBinary) {
    return (
      <div className="file-diff">
        <FileHeader file={file} filePath={filePath} collapsed={false} onToggle={() => undefined} commentCount={commentCount} reviewed={reviewed} onToggleReviewed={handleToggleReviewed} />
        <div className="binary-message">Binary file changed</div>
      </div>
    );
  }

  return (
    <div className="file-diff">
      <FileHeader
        file={file}
        filePath={filePath}
        collapsed={collapsed}
        onToggle={() => setCollapsed((c) => !c)}
        commentCount={commentCount}
        onAddFileComment={() => setShowFileForm((s) => !s)}
        reviewed={reviewed}
        onToggleReviewed={handleToggleReviewed}
      />
      {showFileForm && (
        <CommentForm mode="create" onSubmit={handleFileComment} onCancel={() => setShowFileForm(false)} />
      )}
      {fileCommentsOfType.map((c) => (
        <CommentWidget key={c.id} comment={c} isActive={c.id === activeCommentId} scrollDirection={c.id === activeCommentId ? scrollDirection : null} />
      ))}
      {collapsed ? (
        <div className="collapsed-message">
          <button className="show-diff-btn" onClick={() => setCollapsed(false)}>
            Show full diff ({file.additions + file.deletions} lines)
          </button>
        </div>
      ) : fileDiff && poolReady ? (
        <PierreFileDiff<DiffAnnotationMetadata>
          fileDiff={fileDiff}
          options={{
            themeType,
            theme: syntaxTheme,
            diffStyle: viewType,
            disableFileHeader: true,
            overflow: 'scroll',
            lineDiffType: 'word',
            hunkSeparators: 'line-info',
            enableLineSelection: true,
            enableGutterUtility: true,
            // Must be enabled here AND in the worker init options, or token
            // interactions silently no-op.
            useTokenTransformer: true,
            onLineSelectionEnd,
            onGutterUtilityClick,
          }}
          lineAnnotations={annotations}
          renderAnnotation={renderAnnotation}
          disableWorkerPool
        />
      ) : fileDiff ? (
        <div className="diff-loading">Loading…</div>
      ) : (
        <div className="empty-diff">No changes</div>
      )}
    </div>
  );
}

interface FileHeaderProps {
  file: ParsedFileDiff;
  filePath: string;
  collapsed: boolean;
  onToggle: () => void;
  commentCount: number;
  onAddFileComment?: () => void;
  reviewed: boolean;
  onToggleReviewed: () => void;
}

function FileHeader({ file, filePath, collapsed, onToggle, commentCount, onAddFileComment, reviewed, onToggleReviewed }: FileHeaderProps): React.JSX.Element {
  return (
    <div className="file-header-row">
      <button className="file-header" onClick={onToggle} aria-expanded={!collapsed} type="button">
        <span className="collapse-icon" aria-hidden="true">
          {collapsed ? <ChevronRight size={14} /> : <ChevronDown size={14} />}
        </span>
        <span className={`badge badge-${file.status}`}>{file.status}</span>
        <span className="file-path">{filePath}</span>
        {reviewed && <Check className="reviewed-badge" size={14} aria-label="Reviewed" />}
        {commentCount > 0 && (
          <span className="comment-count-badge" aria-label={`${commentCount} comments`}>
            <MessageSquare size={12} aria-hidden="true" /> {commentCount}
          </span>
        )}
        <span className="line-counts">
          <span className="additions">+{file.additions}</span>
          <span className="deletions">-{file.deletions}</span>
        </span>
      </button>
      <button
        type="button"
        className={`btn btn-review-toggle ${reviewed ? 'btn-reviewed' : ''}`}
        onClick={(e) => { e.stopPropagation(); onToggleReviewed(); }}
        aria-label={reviewed ? 'Unmark as reviewed' : 'Mark as reviewed'}
        title={reviewed ? 'Unmark as reviewed' : 'Mark as reviewed'}
      >
        {reviewed ? <Check size={15} aria-hidden="true" /> : <Circle size={15} aria-hidden="true" />}
      </button>
      {onAddFileComment && (
        <button
          type="button"
          className="btn btn-file-comment"
          onClick={(e) => { e.stopPropagation(); onAddFileComment(); }}
          aria-label="Add file-level comment"
          title="Add file-level comment"
        >
          <MessageSquarePlus size={15} aria-hidden="true" />
        </button>
      )}
    </div>
  );
}
