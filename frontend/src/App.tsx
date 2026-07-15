import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import type { ViewType } from 'react-diff-view';
import type { ReviewMetadata, DiffResponse, ParsedFileDiff, UserPreferences, FinishResponse, FileChange, Comment, ThemeId } from '../../shared/types.js';
import { THEMES, DEFAULT_THEME, normalizeThemeId } from './themes.js';
import { ReviewStoreProvider, useReviewStore } from './hooks/useReviewStore.js';
import { DiffView } from './components/DiffView.js';
import type { DiffViewHandle } from './components/DiffView.js';
import { Sidebar } from './components/Sidebar.js';
import { OverallComments } from './components/OverallComments.js';
import { SummaryPage } from './components/SummaryPage.js';
import { generateClientMarkdown, downloadMarkdown } from './utils/client-markdown.js';
import { cleanExpiredSessions } from './hooks/useSession.js';
import { useQuotaMonitor } from './hooks/useQuotaMonitor.js';
import { useKeyboardShortcuts, SHORTCUT_LIST } from './hooks/useKeyboardShortcuts.js';
import 'react-diff-view/style/index.css';
import './App.css';

type LoadState = 'loading' | 'ready' | 'error';
type AppView = 'review' | 'summary';

const PREFS_KEY = 'local-review:preferences';

function ErrorToast({ message, onDismiss }: { message: string; onDismiss: () => void }): React.JSX.Element {
  useEffect(() => {
    const timer = setTimeout(onDismiss, 5000);
    return () => clearTimeout(timer);
  }, [onDismiss]);

  return (
    <div className="error-toast" role="alert">
      <span>{message}</span>
      <button type="button" onClick={onDismiss} aria-label="Dismiss">✕</button>
    </div>
  );
}

function loadPreferences(): UserPreferences {
  try {
    const raw = localStorage.getItem(PREFS_KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as Partial<UserPreferences>;
      return { theme: normalizeThemeId(parsed.theme) };
    }
  } catch { /* ignore */ }
  return { theme: DEFAULT_THEME };
}

function savePreferences(prefs: UserPreferences): void {
  localStorage.setItem(PREFS_KEY, JSON.stringify(prefs));
}

function useResponsiveViewType(initial: ViewType): [ViewType, (vt: ViewType) => void] {
  const isNarrow = () => window.innerWidth < 1024;
  const [userChoice, setUserChoice] = useState<ViewType>(initial);
  const [effective, setEffective] = useState<ViewType>(isNarrow() ? 'unified' : initial);

  useEffect(() => {
    const onResize = () => setEffective(isNarrow() ? 'unified' : userChoice);
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  }, [userChoice]);

  const setViewType = useCallback((vt: ViewType) => {
    setUserChoice(vt);
    setEffective(isNarrow() ? 'unified' : vt);
  }, []);

  return [effective, setViewType];
}

function DiscardButton(): React.JSX.Element | null {
  const { comments, discardReview } = useReviewStore();
  const [showConfirm, setShowConfirm] = useState(false);

  if (comments.length === 0) return null;

  return (
    <>
      <button className="btn-discard" onClick={() => setShowConfirm(true)} type="button" title="Discard review">
        🗑 Discard
      </button>
      {showConfirm && (
        <div className="discard-overlay" role="dialog" aria-modal="true" aria-label="Confirm discard">
          <div className="discard-dialog">
            <p>This will delete all {comments.length} comment{comments.length !== 1 ? 's' : ''} and reset review status. Are you sure?</p>
            <div className="discard-dialog-actions">
              <button className="btn btn-cancel" onClick={() => setShowConfirm(false)} type="button">Cancel</button>
              <button className="btn btn-danger" onClick={() => { discardReview(); setShowConfirm(false); }} type="button">Discard</button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}

interface SummaryData {
  markdown: string;
  outputPath: string;
}

function DoneButton({ metadata, onFinish }: { metadata: ReviewMetadata; onFinish: (data: SummaryData) => void }): React.JSX.Element {
  const { comments, reviewedFiles } = useReviewStore();
  const [showConfirm, setShowConfirm] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [exportError, setExportError] = useState('');

  const fileCount = new Set(comments.filter((c) => c.filePath).map((c) => c.filePath)).size;

  const handleExport = useCallback(async () => {
    setExporting(true);
    setExportError('');
    try {
      const res = await fetch('/api/v1/finish', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'X-CSRF-Token': metadata.csrfToken },
        body: JSON.stringify({
          comments,
          reviewedFiles,
          metadata: { commitRange: metadata.commitRange, timestamp: new Date().toISOString() },
          _csrf: metadata.csrfToken,
        }),
      });
      if (!res.ok) throw new Error(`Server returned ${res.status}`);
      const data = await res.json() as FinishResponse;
      setShowConfirm(false);
      onFinish({ markdown: data.markdown, outputPath: data.outputPath });
    } catch {
      // Client-side fallback
      const fallbackMd = generateClientMarkdown(comments);
      downloadMarkdown(fallbackMd);
      setExportError('Server error — markdown generated locally');
      setExporting(false);
    }
  }, [comments, reviewedFiles, metadata, onFinish]);

  return (
    <>
      <button className="btn btn-done" onClick={() => setShowConfirm(true)} type="button">
        ✓ Done
      </button>
      {showConfirm && (
        <div className="modal-overlay" role="dialog" aria-modal="true" aria-label="Export confirmation">
          <div className="modal-dialog">
            <h2>Export {comments.length} comment{comments.length !== 1 ? 's' : ''} to markdown?</h2>
            <p className="modal-detail">
              {comments.length} comment{comments.length !== 1 ? 's' : ''} across {fileCount} file{fileCount !== 1 ? 's' : ''}
            </p>
            {exportError && <p className="export-error" role="alert">{exportError}</p>}
            <div className="modal-actions">
              <button className="btn btn-cancel" onClick={() => { setShowConfirm(false); setExportError(''); setExporting(false); }} type="button" disabled={exporting}>
                Cancel
              </button>
              <button className="btn btn-submit" onClick={handleExport} type="button" disabled={exporting}>
                {exporting ? 'Exporting...' : 'Export'}
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}

function ShortcutHelpModal({ onClose }: { onClose: () => void }): React.JSX.Element {
  return (
    <div className="modal-overlay" role="dialog" aria-modal="true" aria-label="Keyboard shortcuts" onClick={onClose}>
      <div className="shortcut-help-dialog" onClick={(e) => e.stopPropagation()}>
        <div className="shortcut-help-header">
          <h2>Keyboard Shortcuts</h2>
          <button type="button" className="btn-icon" onClick={onClose} aria-label="Close">✕</button>
        </div>
        <table className="shortcut-table">
          <tbody>
            {SHORTCUT_LIST.map((s) => (
              <tr key={s.key}>
                <td><kbd>{s.key}</kbd></td>
                <td>{s.description}</td>
              </tr>
            ))}
          </tbody>
        </table>
        <p className="shortcut-help-note">Shortcuts are disabled when typing in a text field.</p>
      </div>
    </div>
  );
}

function StatusBar({ totalFiles }: { totalFiles: number }): React.JSX.Element {
  const { comments, reviewedFiles } = useReviewStore();

  const categoryBreakdown = useMemo(() => {
    const counts: Record<Comment['category'], number> = { fix: 0, suggestion: 0, question: 0, nit: 0 };
    for (const c of comments) counts[c.category]++;
    return counts;
  }, [comments]);

  return (
    <div className="status-bar" role="status">
      <span>{comments.length} comment{comments.length !== 1 ? 's' : ''}</span>
      <span className="status-separator">|</span>
      <span>{reviewedFiles.length}/{totalFiles} files reviewed</span>
      <span className="status-separator">|</span>
      <span className="status-categories">
        fix: {categoryBreakdown.fix} • suggestion: {categoryBreakdown.suggestion} • question: {categoryBreakdown.question} • nit: {categoryBreakdown.nit}
      </span>
    </div>
  );
}

function AppContent({
  metadata,
  diffFiles,
  fileChanges,
  onFinish,
  onRefresh,
  onSelectTheme,
  theme,
}: {
  metadata: ReviewMetadata;
  diffFiles: ParsedFileDiff[];
  fileChanges: FileChange[];
  onFinish: (data: SummaryData) => void;
  onRefresh: () => void;
  onSelectTheme: (id: ThemeId) => void;
  theme: ThemeId;
}): React.JSX.Element {
  const { viewMode, setViewMode, comments } = useReviewStore();
  const [viewType, setViewType] = useResponsiveViewType(viewMode as ViewType);
  const quota = useQuotaMonitor();
  const diffViewRef = useRef<DiffViewHandle>(null);
  const [showHelp, setShowHelp] = useState(false);
  const [showRefreshWarn, setShowRefreshWarn] = useState(false);
  const [commentIdx, setCommentIdx] = useState(-1);
  const [activeCommentId, setActiveCommentId] = useState<string | null>(null);
  const [scrollDirection, setScrollDirection] = useState<'forward' | 'backward' | null>(null);

  useEffect(() => {
    setViewType(viewMode as ViewType);
  }, [viewMode, setViewType]);

  const handleToggleView = useCallback(() => {
    const next = viewType === 'split' ? 'unified' : 'split';
    setViewType(next);
    setViewMode(next);
  }, [viewType, setViewType, setViewMode]);

  const handleFileClick = useCallback((filePath: string) => {
    diffViewRef.current?.scrollToFile(filePath);
  }, []);

  // Sorted comments for navigation — matches visual order on page
  const sortedComments = useMemo(() => {
    const fileOrder = new Map<string, number>();
    diffFiles.forEach((f, i) => fileOrder.set(f.newPath || f.oldPath, i));

    return [...comments].sort((a, b) => {
      // Overall comments first
      if (a.type === 'overall' && b.type !== 'overall') return -1;
      if (a.type !== 'overall' && b.type === 'overall') return 1;
      // Then by file position in diff view
      const aIdx = fileOrder.get(a.filePath ?? '') ?? Infinity;
      const bIdx = fileOrder.get(b.filePath ?? '') ?? Infinity;
      if (aIdx !== bIdx) return aIdx - bIdx;
      // File-level comments before line comments in same file
      if (a.type === 'file' && b.type !== 'file') return -1;
      if (a.type !== 'file' && b.type === 'file') return 1;
      // Then by start line
      return (a.startLine ?? 0) - (b.startLine ?? 0);
    });
  }, [comments, diffFiles]);

  const isFirstComment = commentIdx <= 0;
  const isLastComment = commentIdx >= sortedComments.length - 1;

  const navigateComment = useCallback((direction: 1 | -1) => {
    if (sortedComments.length === 0) return;
    const next = Math.max(0, Math.min(sortedComments.length - 1, commentIdx + direction));
    setCommentIdx(next);
    const c = sortedComments[next];
    setActiveCommentId(c.id);
    setScrollDirection(direction === 1 ? 'forward' : 'backward');
    // scrollIntoView in CommentWidget handles the actual scrolling.
    // We only need scrollToFile to ensure the virtualizer renders the file's DOM.
    // Use a flag to skip the file-level scroll and let the widget scroll precisely.
    if (c.type === 'overall') {
      document.querySelector('.diff-view-scroll')?.scrollTo({ top: 0 });
    } else if (c.filePath) {
      // Ensure the file is rendered by the virtualizer, then let CommentWidget scroll
      diffViewRef.current?.scrollToFile(c.filePath);
    }
  }, [sortedComments, commentIdx]);

  const navigateFile = useCallback((direction: 1 | -1) => {
    if (!diffFiles.length) return;
    // Find current visible file index — just cycle through
    const fileNames = diffFiles.map((f) => f.newPath || f.oldPath);
    // Simple: scroll to next/prev from start
    const el = document.querySelector('.diff-view-scroll, .app-main');
    const scrollTop = el?.scrollTop ?? 0;
    // Use diffViewRef to scroll by index
    const currentIdx = Math.max(0, fileNames.findIndex((_, i) => {
      const node = document.querySelector(`[data-index="${i}"]`);
      if (!node) return false;
      return (node as HTMLElement).offsetTop >= scrollTop;
    }));
    const nextIdx = Math.max(0, Math.min(diffFiles.length - 1, currentIdx + direction));
    const target = fileNames[nextIdx];
    if (target) diffViewRef.current?.scrollToFile(target);
  }, [diffFiles]);

  const handleAddOverallComment = useCallback(() => {
    // Scroll to top where overall comments section is
    const el = document.querySelector('.app-main');
    if (el) el.scrollTop = 0;
    // Click the add button if it exists
    const btn = document.querySelector('.overall-comments .btn-add') as HTMLButtonElement | null;
    btn?.click();
  }, []);

  useKeyboardShortcuts({
    nextFile: () => navigateFile(1),
    prevFile: () => navigateFile(-1),
    nextComment: () => navigateComment(1),
    prevComment: () => navigateComment(-1),
    addComment: handleAddOverallComment,
    toggleViewMode: handleToggleView,
    closeForm: () => {
      if (showHelp) { setShowHelp(false); return; }
      // Try to click any visible cancel button in comment forms
      const cancelBtn = document.querySelector('.comment-form .btn-cancel') as HTMLButtonElement | null;
      cancelBtn?.click();
    },
    toggleHelp: () => setShowHelp((v) => !v),
  });

  const handleRefresh = useCallback(() => {
    setShowRefreshWarn(true);
  }, []);

  const confirmRefresh = useCallback(() => {
    setShowRefreshWarn(false);
    onRefresh();
  }, [onRefresh]);

  return (
    <div className="app app-with-sidebar">
      {quota.isNearQuota && (
        <div className="quota-warning" role="alert">
          ⚠️ localStorage is {quota.usagePercent}% full. Consider discarding old reviews.
        </div>
      )}
      <header className="header">
        <div className="header-row">
          <h1>
            Reviewing <code>{metadata.repoName}</code>:{' '}
            <code>{metadata.baseRef}..{metadata.headRef}</code>
          </h1>
          <div className="toolbar" role="toolbar" aria-label="Review toolbar">
            <div className="toolbar-group">
              <button className="btn toolbar-btn" onClick={() => navigateComment(-1)} type="button" title="Previous comment (k)" disabled={sortedComments.length === 0 || isFirstComment}>
                ← Comment
              </button>
              <button className="btn toolbar-btn" onClick={() => navigateComment(1)} type="button" title="Next comment (j)" disabled={sortedComments.length === 0 || isLastComment}>
                Comment →
              </button>
            </div>
            <div className="toolbar-separator" />
            <div className="toolbar-group">
              <button className="btn toolbar-btn" onClick={handleAddOverallComment} type="button" title="Add overall comment (c)">
                💬 Comment
              </button>
              <button className="btn toolbar-btn" onClick={handleRefresh} type="button" title="Refresh diff">
                🔄 Refresh
              </button>
            </div>
            <div className="toolbar-separator" />
            <div className="toolbar-group">
              <button className="view-toggle" onClick={handleToggleView} type="button" title="Toggle view mode (d)">
                {viewType === 'split' ? '⇔ Split' : '≡ Unified'}
              </button>
              <select
                className="theme-select"
                value={theme}
                onChange={(e) => onSelectTheme(e.target.value as ThemeId)}
                aria-label="Select color theme"
                title="Color theme"
              >
                <optgroup label="Dark">
                  {THEMES.filter((t) => t.mode === 'dark').map((t) => (
                    <option key={t.id} value={t.id}>{t.label}</option>
                  ))}
                </optgroup>
                <optgroup label="Light">
                  {THEMES.filter((t) => t.mode === 'light').map((t) => (
                    <option key={t.id} value={t.id}>{t.label}</option>
                  ))}
                </optgroup>
              </select>
              <button className="btn toolbar-btn toolbar-help-btn" onClick={() => setShowHelp(true)} type="button" title="Keyboard shortcuts (?)">
                ?
              </button>
            </div>
            <div className="toolbar-separator" />
            <div className="toolbar-group">
              <DiscardButton />
              <DoneButton metadata={metadata} onFinish={onFinish} />
            </div>
          </div>
        </div>
      </header>
      <div className="app-body">
        <Sidebar files={fileChanges} onFileClick={handleFileClick} />
        <div className="app-main">
          <OverallComments activeCommentId={activeCommentId} scrollDirection={scrollDirection} />
          <DiffView ref={diffViewRef} files={diffFiles} viewType={viewType} activeCommentId={activeCommentId} scrollDirection={scrollDirection} />
        </div>
      </div>
      <StatusBar totalFiles={fileChanges.length} />
      {showHelp && <ShortcutHelpModal onClose={() => setShowHelp(false)} />}
      {showRefreshWarn && (
        <div className="modal-overlay" role="dialog" aria-modal="true" aria-label="Refresh warning">
          <div className="modal-dialog">
            <h2>Refresh diff?</h2>
            <p className="modal-detail">
              Line numbers may shift. Comments may become misaligned.
            </p>
            <div className="modal-actions">
              <button className="btn btn-cancel" onClick={() => setShowRefreshWarn(false)} type="button">Cancel</button>
              <button className="btn btn-submit" onClick={confirmRefresh} type="button">Refresh</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export function App(): React.JSX.Element {
  const [metadata, setMetadata] = useState<ReviewMetadata | null>(null);
  const [diffFiles, setDiffFiles] = useState<ParsedFileDiff[] | null>(null);
  const [loadState, setLoadState] = useState<LoadState>('loading');
  const [error, setError] = useState('');
  const [toast, setToast] = useState('');
  const [theme, setTheme] = useState<ThemeId>(() => loadPreferences().theme);
  const [view, setView] = useState<AppView>('review');
  const [summaryData, setSummaryData] = useState<SummaryData | null>(null);

  useEffect(() => {
    cleanExpiredSessions();
  }, []);

  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme);
    savePreferences({ theme });
  }, [theme]);

  const fetchData = useCallback(() => {
    setLoadState('loading');
    setError('');
    Promise.all([
      fetch('/api/v1/metadata').then((r) => {
        if (!r.ok) throw new Error(`metadata: ${r.status}`);
        return r.json() as Promise<ReviewMetadata>;
      }),
      fetch('/api/v1/diff').then((r) => {
        if (!r.ok) throw new Error(`diff: ${r.status}`);
        return r.json() as Promise<DiffResponse>;
      }),
    ])
      .then(([meta, diff]) => {
        setMetadata(meta);
        setDiffFiles(diff.files ?? []);
        setLoadState('ready');
      })
      .catch((err: Error) => {
        setError(err.message);
        setLoadState('error');
      });
  }, []);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  const handleRefresh = useCallback(() => {
    // Re-fetch only diff data
    fetch('/api/v1/diff')
      .then((r) => {
        if (!r.ok) throw new Error(`diff: ${r.status}`);
        return r.json() as Promise<DiffResponse>;
      })
      .then((diff) => {
        setDiffFiles(diff.files ?? []);
      })
      .catch((err: Error) => {
        setToast(`Refresh failed: ${err.message}`);
      });
  }, []);

  const handleFinish = useCallback((data: SummaryData) => {
    setSummaryData(data);
    setView('summary');
  }, []);

  const handleContinue = useCallback(() => {
    setView('review');
  }, []);

  const handleSelectTheme = useCallback((id: ThemeId) => {
    setTheme(id);
  }, []);

  if (loadState === 'loading') {
    return (
      <div className="app skeleton" role="status" aria-label="Loading">
        <div className="skeleton-line" />
        <div className="skeleton-line" />
        <div className="skeleton-line" />
        <div className="skeleton-line" />
      </div>
    );
  }

  if (loadState === 'error' || !metadata || !diffFiles) {
    return (
      <div className="app error">
        <p>Failed to load review data: {error}</p>
        <button className="btn" onClick={fetchData} type="button">Retry</button>
      </div>
    );
  }

  return (
    <ReviewStoreProvider metadata={metadata}>
      {toast && <ErrorToast message={toast} onDismiss={() => setToast('')} />}
      {view === 'summary' && summaryData ? (
        <SummaryPage
          markdown={summaryData.markdown}
          outputPath={summaryData.outputPath}
          csrfToken={metadata.csrfToken}
          onContinue={handleContinue}
        />
      ) : (
        <AppContent
          metadata={metadata}
          diffFiles={diffFiles}
          fileChanges={metadata.files}
          onFinish={handleFinish}
          onRefresh={handleRefresh}
          onSelectTheme={handleSelectTheme}
          theme={theme}
        />
      )}
    </ReviewStoreProvider>
  );
}
