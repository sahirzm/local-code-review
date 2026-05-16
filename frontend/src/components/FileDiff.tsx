import { useState, useEffect, useMemo, useCallback } from 'react';
import { Diff, Hunk, tokenize, markEdits, getChangeKey } from 'react-diff-view';
import type { ViewType, HunkTokens, EventMap, ChangeEventArgs } from 'react-diff-view';
import { refractor } from 'refractor';
import type { ParsedFileDiff, Comment } from '../../../shared/types.js';
import { transformFile } from '../utils/transform-diff.js';
import { useReviewStore } from '../hooks/useReviewStore.js';
import { CommentForm } from './CommentForm.js';
import { CommentWidget } from './CommentWidget.js';

const EXT_TO_LANG: Record<string, string> = {
  ts: 'typescript', tsx: 'tsx', js: 'javascript', jsx: 'jsx',
  json: 'json', css: 'css', html: 'markup', md: 'markdown',
  yaml: 'yaml', yml: 'yaml', py: 'python', rs: 'rust',
  go: 'go', java: 'java', sh: 'bash', bash: 'bash',
  scss: 'scss', less: 'less', sql: 'sql', xml: 'markup',
  rb: 'ruby', php: 'php', c: 'c', cpp: 'cpp', h: 'c',
};

function getLanguage(filePath: string): string | null {
  const ext = filePath.split('.').pop()?.toLowerCase() ?? '';
  const lang = EXT_TO_LANG[ext];
  if (lang && refractor.registered(lang)) return lang;
  return null;
}

interface FileDiffProps {
  file: ParsedFileDiff;
  viewType: ViewType;
  activeCommentId?: string | null;
  scrollDirection?: 'forward' | 'backward' | null;
}

interface ActiveForm {
  changeKey: string;
  line: number;
  endLine?: number;
  side: 'old' | 'new';
}

export function FileDiff({ file, viewType, activeCommentId, scrollDirection }: FileDiffProps): React.JSX.Element {
  const isLargeDefault = file.isLarge;
  const { addComment, getCommentsForFile, isFileReviewed, markFileReviewed, unmarkFileReviewed } = useReviewStore();
  const filePath = file.newPath || file.oldPath;
  const reviewed = isFileReviewed(filePath);
  const [collapsed, setCollapsed] = useState(isLargeDefault || reviewed);

  // Auto-expand if this file contains the active comment
  const fileComments = getCommentsForFile(filePath);
  const hasActiveComment = activeCommentId != null && fileComments.some((c) => c.id === activeCommentId);
  useEffect(() => {
    if (hasActiveComment && collapsed) setCollapsed(false);
  }, [hasActiveComment, collapsed]);
  const [activeForm, setActiveForm] = useState<ActiveForm | null>(null);
  const [showFileForm, setShowFileForm] = useState(false);
  const [rangeStart, setRangeStart] = useState<{ line: number; side: 'old' | 'new'; changeKey: string } | null>(null);

  const fileData = useMemo(() => transformFile(file), [file]);

  const language = useMemo(() => getLanguage(filePath), [filePath]);

  const tokens = useMemo((): HunkTokens | null => {
    if (fileData.hunks.length === 0) return null;
    const enhancers = [markEdits(fileData.hunks, { type: 'block' })];
    try {
      if (language !== null) {
        return tokenize(fileData.hunks, { highlight: true, refractor, language, enhancers });
      }
      return tokenize(fileData.hunks, { enhancers });
    } catch {
      return null;
    }
  }, [fileData.hunks, language]);

  const commentCount = fileComments.length;

  const [hoverLine, setHoverLine] = useState<number | null>(null);

  // Compute selected change keys — highlight range during selection and while form is open
  const selectedChanges = useMemo((): string[] => {
    // Use rangeStart for hover preview, or activeForm.endLine for confirmed range with form open
    const anchor = rangeStart ?? (activeForm?.endLine != null ? { line: activeForm.line, side: activeForm.side } : null);
    if (!anchor) return [];

    const endLine = rangeStart ? (hoverLine ?? anchor.line) : (activeForm?.endLine ?? anchor.line);
    const lo = Math.min(anchor.line, endLine);
    const hi = Math.max(anchor.line, endLine);

    const keys: string[] = [];
    for (const hunk of fileData.hunks) {
      for (const change of hunk.changes) {
        let lineNum: number | undefined;
        let changeSide: 'old' | 'new' | undefined;
        if (change.type === 'delete') { lineNum = change.lineNumber; changeSide = 'old'; }
        else if (change.type === 'insert') { lineNum = change.lineNumber; changeSide = 'new'; }
        else {
          lineNum = anchor.side === 'old' ? change.oldLineNumber : change.newLineNumber;
          changeSide = anchor.side;
        }
        if (changeSide === anchor.side && lineNum !== undefined && lineNum >= lo && lineNum <= hi) {
          keys.push(getChangeKey(change));
        }
      }
    }
    return keys;
  }, [rangeStart, hoverLine, activeForm, fileData.hunks]);
  const widgets = useMemo(() => {
    const map: Record<string, React.ReactNode> = {};

    // Map line/range comments to change keys
    const lineComments = fileComments.filter((c) => c.type === 'line' || c.type === 'range');
    for (const hunk of fileData.hunks) {
      for (const change of hunk.changes) {
        const key = getChangeKey(change);
        let lineNum: number;
        let side: 'old' | 'new';

        if (change.type === 'delete') {
          lineNum = change.lineNumber;
          side = 'old';
        } else if (change.type === 'insert') {
          lineNum = change.lineNumber;
          side = 'new';
        } else {
          // normal: check both sides
          const oldMatches = lineComments.filter(
            (c) => c.side === 'old' && c.startLine !== undefined && (c.type === 'range' ? (c.endLine ?? c.startLine) === change.oldLineNumber : c.startLine === change.oldLineNumber),
          );
          const newMatches = lineComments.filter(
            (c) => c.side === 'new' && c.startLine !== undefined && (c.type === 'range' ? (c.endLine ?? c.startLine) === change.newLineNumber : c.startLine === change.newLineNumber),
          );
          const matches = [...oldMatches, ...newMatches];
          if (matches.length > 0 || (activeForm && activeForm.changeKey === key)) {
            map[key] = (
              <div className="comment-widget-container">
                {matches.map((c) => <CommentWidget key={c.id} comment={c} isActive={c.id === activeCommentId} scrollDirection={c.id === activeCommentId ? scrollDirection : null} />)}
                {activeForm?.changeKey === key && (
                  <CommentForm
                    mode="create"
                    onSubmit={(text: string, category: Comment['category']) => {
                      const isRange = activeForm.endLine !== undefined && activeForm.endLine !== activeForm.line;
                      addComment({
                        type: isRange ? 'range' : 'line',
                        category,
                        text,
                        filePath,
                        startLine: activeForm.line,
                        endLine: activeForm.endLine ?? activeForm.line,
                        side: activeForm.side,
                      });
                      setActiveForm(null);
                      setRangeStart(null);
                    }}
                    onCancel={() => { setActiveForm(null); setRangeStart(null); }}
                  />
                )}
              </div>
            );
          }
          continue;
        }

        lineNum = lineNum!;
        side = side!;
        const matches = lineComments.filter(
          (c) => c.side === side && c.startLine !== undefined && (c.type === 'range' ? (c.endLine ?? c.startLine) === lineNum : c.startLine === lineNum),
        );

        if (matches.length > 0 || (activeForm && activeForm.changeKey === key)) {
          map[key] = (
            <div className="comment-widget-container">
              {matches.map((c) => <CommentWidget key={c.id} comment={c} isActive={c.id === activeCommentId} scrollDirection={c.id === activeCommentId ? scrollDirection : null} />)}
              {activeForm?.changeKey === key && (
                <CommentForm
                  mode="create"
                  onSubmit={(text: string, category: Comment['category']) => {
                    const isRange = activeForm.endLine !== undefined && activeForm.endLine !== activeForm.line;
                    addComment({
                      type: isRange ? 'range' : 'line',
                      category,
                      text,
                      filePath,
                      startLine: activeForm.line,
                      endLine: activeForm.endLine ?? activeForm.line,
                      side: activeForm.side,
                    });
                    setActiveForm(null);
                    setRangeStart(null);
                  }}
                  onCancel={() => { setActiveForm(null); setRangeStart(null); }}
                />
              )}
            </div>
          );
        }
      }
    }

    return map;
  }, [fileComments, fileData.hunks, activeForm, addComment, filePath]);

  const gutterEvents: EventMap = useMemo(
    () => ({
      onClick: (arg: ChangeEventArgs, e?: React.MouseEvent) => {
        const change = arg.change;
        if (!change) return;
        const key = getChangeKey(change);
        let line: number;
        let side: 'old' | 'new';
        if (change.type === 'delete') {
          line = change.lineNumber;
          side = 'old';
        } else if (change.type === 'insert') {
          line = change.lineNumber;
          side = 'new';
        } else {
          line = change.newLineNumber;
          side = 'new';
        }

        const shiftKey = e?.shiftKey ?? false;

        // Shift-click: complete a range selection
        if (shiftKey && rangeStart && rangeStart.side === side) {
          const startLine = Math.min(rangeStart.line, line);
          const endLine = Math.max(rangeStart.line, line);
          setActiveForm({ changeKey: key, line: startLine, endLine, side });
          setHoverLine(null);
          setRangeStart(null);
          return;
        }

        // Regular click: start a potential range or open single-line form
        if (activeForm?.changeKey === key) {
          setActiveForm(null);
          setRangeStart(null);
          setHoverLine(null);
        } else {
          setRangeStart({ line, side, changeKey: key });
          setActiveForm({ changeKey: key, line, side });
          setHoverLine(null);
        }
      },
      onMouseEnter: (arg: ChangeEventArgs) => {
        if (!rangeStart || !arg.change) return;
        const change = arg.change;
        let line: number;
        if (change.type === 'delete') { line = change.lineNumber; }
        else if (change.type === 'insert') { line = change.lineNumber; }
        else { line = change.newLineNumber; }
        setHoverLine(line);
      },
    }),
    [rangeStart, activeForm],
  );

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
      ) : fileData.hunks.length > 0 ? (
        <>
          {rangeStart && (
            <div className="range-mode-banner">
              ⇧ Shift+click another line to select range from line {rangeStart.line} · <button type="button" className="range-cancel-btn" onClick={() => { setRangeStart(null); setActiveForm(null); }}>Cancel</button>
            </div>
          )}
          <Diff
            viewType={viewType}
            diffType={fileData.type}
            hunks={fileData.hunks}
            tokens={tokens}
            widgets={widgets}
            gutterEvents={gutterEvents}
            selectedChanges={selectedChanges}
          >
            {(hunks) => hunks.map((hunk) => <Hunk key={hunk.content} hunk={hunk} />)}
          </Diff>
        </>
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
        <span className="collapse-icon" aria-hidden="true">{collapsed ? '▶' : '▼'}</span>
        <span className={`badge badge-${file.status}`}>{file.status}</span>
        <span className="file-path">{filePath}</span>
        {reviewed && <span className="reviewed-badge" aria-label="Reviewed">✓</span>}
        {commentCount > 0 && (
          <span className="comment-count-badge" aria-label={`${commentCount} comments`}>
            💬 {commentCount}
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
        {reviewed ? '✓' : '○'}
      </button>
      {onAddFileComment && (
        <button
          type="button"
          className="btn btn-file-comment"
          onClick={(e) => { e.stopPropagation(); onAddFileComment(); }}
          aria-label="Add file-level comment"
          title="Add file-level comment"
        >
          💬+
        </button>
      )}
    </div>
  );
}
