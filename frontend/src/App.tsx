import { useState, useEffect, useCallback, useMemo } from 'react';
import {
  Trash2, Check, Circle, MessageSquarePlus, MessageSquare, RefreshCw, Columns2, AlignJustify,
  ChevronLeft, ChevronRight, HelpCircle, X, AlertTriangle, Settings, FileText,
} from 'lucide-react';
import { Toaster, toast } from 'sonner';
import { Modal } from './components/ui/Modal.js';
import { TooltipProvider, Tooltip } from './components/ui/Tooltip.js';
import type { ReviewMetadata, DiffResponse, ParsedFileDiff, UserPreferences, FinishResponse, FileChange, Comment, ThemeId, CodeFontId, UiFontId } from '../../shared/types.js';
import { THEMES, DEFAULT_THEME, normalizeThemeId } from './themes.js';
import {
  CODE_FONTS, UI_FONTS, DEFAULT_CODE_FONT, DEFAULT_UI_FONT,
  normalizeCodeFontId, normalizeUiFontId, resolveCodeFontStack, resolveUiFontStack,
} from './fonts.js';
import { ReviewStoreProvider, useReviewStore } from './hooks/useReviewStore.js';
import { DiffView } from './components/DiffView.js';
import type { PierreViewType } from './components/FileDiff.js';
import { DiffWorkerPoolProvider, useWorkerPoolThemeSync } from './components/diff/workerPool.js';
import { resolveShikiTheme } from './components/diff/shikiTheme.js';
import { Sidebar } from './components/Sidebar.js';
import { OverallComments } from './components/OverallComments.js';
import { SummaryPage } from './components/SummaryPage.js';
import { generateClientMarkdown, downloadMarkdown } from './utils/client-markdown.js';
import { cleanExpiredSessions } from './hooks/useSession.js';
import { useQuotaMonitor } from './hooks/useQuotaMonitor.js';
import { useKeyboardShortcuts, SHORTCUT_LIST } from './hooks/useKeyboardShortcuts.js';
import './App.css';

type LoadState = 'loading' | 'ready' | 'error';
type AppView = 'review' | 'summary';
type ViewType = PierreViewType;

/** Diff context-line levels offered in the toolbar; 'full' maps to whole-file. */
const CONTEXT_LEVELS = [5, 10, 20, 50, 'full'] as const;
type ContextLevel = (typeof CONTEXT_LEVELS)[number];
const DEFAULT_CONTEXT: ContextLevel = 5;
const CONTEXT_STORAGE_KEY = 'local-review:diff-context';

function loadContextLevel(): ContextLevel {
  try {
    const raw = localStorage.getItem(CONTEXT_STORAGE_KEY);
    if (raw === 'full') return 'full';
    const n = Number(raw);
    if (CONTEXT_LEVELS.includes(n as ContextLevel)) return n as ContextLevel;
  } catch { /* ignore */ }
  return DEFAULT_CONTEXT;
}

const PREFS_KEY = 'local-review:preferences';

const MIN_FONT_SIZE = 10;
const MAX_FONT_SIZE = 20;
const DEFAULT_FONT_SIZE = 13;

function clampFontSize(size: number): number {
  if (!Number.isFinite(size)) return DEFAULT_FONT_SIZE;
  return Math.min(MAX_FONT_SIZE, Math.max(MIN_FONT_SIZE, Math.round(size)));
}

function loadPreferences(): UserPreferences {
  try {
    const raw = localStorage.getItem(PREFS_KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as Partial<UserPreferences>;
      return {
        theme: normalizeThemeId(parsed.theme),
        fontSize: clampFontSize(parsed.fontSize ?? DEFAULT_FONT_SIZE),
        codeFont: normalizeCodeFontId(parsed.codeFont),
        uiFont: normalizeUiFontId(parsed.uiFont),
      };
    }
  } catch { /* ignore */ }
  return { theme: DEFAULT_THEME, fontSize: DEFAULT_FONT_SIZE, codeFont: DEFAULT_CODE_FONT, uiFont: DEFAULT_UI_FONT };
}

function savePreferences(prefs: UserPreferences): void {
  localStorage.setItem(PREFS_KEY, JSON.stringify(prefs));
}

// The backend returns files in git's diff order, which doesn't match the
// alphabetical sidebar tree. Sort by display path so the diff view, file
// navigation, and sidebar all agree on order.
export function sortDiffFiles(files: ParsedFileDiff[]): ParsedFileDiff[] {
  return [...files].sort((a, b) =>
    (a.newPath || a.oldPath).localeCompare(b.newPath || b.oldPath),
  );
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
        <Trash2 size={14} aria-hidden="true" /> Discard
      </button>
      <Modal open={showConfirm} onOpenChange={setShowConfirm} ariaLabel="Confirm discard" dialogClassName="discard-dialog">
        <p>This will delete all {comments.length} comment{comments.length !== 1 ? 's' : ''} and reset review status. Are you sure?</p>
        <div className="discard-dialog-actions">
          <button className="btn btn-cancel" onClick={() => setShowConfirm(false)} type="button">Cancel</button>
          <button className="btn btn-danger" onClick={() => { discardReview(); setShowConfirm(false); }} type="button">Discard</button>
        </div>
      </Modal>
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
      <button className="btn btn-done btn-done-icon" onClick={() => setShowConfirm(true)} type="button" aria-label="Done" title="Done">
        <Check size={16} aria-hidden="true" />
      </button>
      <Modal
        open={showConfirm}
        onOpenChange={(o) => { if (!o) { setExportError(''); setExporting(false); } setShowConfirm(o); }}
        ariaLabel="Export confirmation"
        dialogClassName="modal-dialog"
      >
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
      </Modal>
    </>
  );
}

function ShortcutHelpModal({ open, onOpenChange }: { open: boolean; onOpenChange: (open: boolean) => void }): React.JSX.Element {
  return (
    <Modal open={open} onOpenChange={onOpenChange} ariaLabel="Keyboard shortcuts" dialogClassName="shortcut-help-dialog">
      <div className="shortcut-help-header">
        <h2>Keyboard Shortcuts</h2>
        <button type="button" className="btn-icon" onClick={() => onOpenChange(false)} aria-label="Close"><X size={16} aria-hidden="true" /></button>
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
    </Modal>
  );
}

interface SettingsModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  viewType: ViewType;
  onToggleView: () => void;
  theme: ThemeId;
  onSelectTheme: (id: ThemeId) => void;
  contextLevel: ContextLevel;
  onSelectContext: (level: ContextLevel) => void;
  fontSize: number;
  onChangeFontSize: (delta: number) => void;
  codeFont: CodeFontId;
  onSelectCodeFont: (id: CodeFontId) => void;
  uiFont: UiFontId;
  onSelectUiFont: (id: UiFontId) => void;
  onOpenShortcuts: () => void;
}

function SettingsModal({
  open, onOpenChange, viewType, onToggleView, theme, onSelectTheme,
  contextLevel, onSelectContext, fontSize, onChangeFontSize,
  codeFont, onSelectCodeFont, uiFont, onSelectUiFont, onOpenShortcuts,
}: SettingsModalProps): React.JSX.Element {
  return (
    <Modal open={open} onOpenChange={onOpenChange} ariaLabel="Settings" dialogClassName="settings-dialog">
      <div className="settings-header">
        <h2>Settings</h2>
        <button type="button" className="btn-icon" onClick={() => onOpenChange(false)} aria-label="Close"><X size={16} aria-hidden="true" /></button>
      </div>
      <div className="settings-body">
        <div className="settings-row">
          <span className="settings-label" id="settings-view-label">View</span>
          <button className="view-toggle" onClick={onToggleView} type="button" aria-labelledby="settings-view-label" title="Toggle view mode (d)">
            {viewType === 'split'
              ? <><Columns2 size={14} aria-hidden="true" /> Split</>
              : <><AlignJustify size={14} aria-hidden="true" /> Unified</>}
          </button>
        </div>
        <div className="settings-row">
          <label className="settings-label" htmlFor="settings-theme">Theme</label>
          <select
            id="settings-theme"
            className="theme-select"
            value={theme}
            onChange={(e) => onSelectTheme(e.target.value as ThemeId)}
            aria-label="Select color theme"
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
        </div>
        <div className="settings-row">
          <span className="settings-label" id="settings-fontsize-label">Font size</span>
          <div className="font-size-control" role="group" aria-labelledby="settings-fontsize-label">
            <button className="btn toolbar-btn font-size-btn" onClick={() => onChangeFontSize(-1)} type="button" aria-label="Decrease diff font size" title="Decrease font size" disabled={fontSize <= MIN_FONT_SIZE}>
              A−
            </button>
            <span className="font-size-value" aria-live="polite" title="Diff font size">{fontSize}px</span>
            <button className="btn toolbar-btn font-size-btn" onClick={() => onChangeFontSize(1)} type="button" aria-label="Increase diff font size" title="Increase font size" disabled={fontSize >= MAX_FONT_SIZE}>
              A+
            </button>
          </div>
        </div>
        <div className="settings-row">
          <label className="settings-label" htmlFor="settings-code-font">Code font</label>
          <select
            id="settings-code-font"
            className="theme-select"
            value={codeFont}
            onChange={(e) => onSelectCodeFont(e.target.value as CodeFontId)}
            aria-label="Select code font"
          >
            {CODE_FONTS.map((f) => (
              <option key={f.id} value={f.id}>{f.label}</option>
            ))}
          </select>
        </div>
        <div className="settings-row">
          <label className="settings-label" htmlFor="settings-ui-font">UI font</label>
          <select
            id="settings-ui-font"
            className="theme-select"
            value={uiFont}
            onChange={(e) => onSelectUiFont(e.target.value as UiFontId)}
            aria-label="Select UI font"
          >
            {UI_FONTS.map((f) => (
              <option key={f.id} value={f.id}>{f.label}</option>
            ))}
          </select>
        </div>
        <div className="settings-row">
          <label className="settings-label" htmlFor="settings-context">Context lines</label>
          <select
            id="settings-context"
            className="context-select"
            value={String(contextLevel)}
            onChange={(e) => onSelectContext(e.target.value === 'full' ? 'full' : Number(e.target.value) as ContextLevel)}
            aria-label="Diff context lines"
          >
            {CONTEXT_LEVELS.map((level) => (
              <option key={level} value={String(level)}>
                {level === 'full' ? 'Full context' : `${level} lines`}
              </option>
            ))}
          </select>
        </div>
      </div>
      <div className="settings-footer">
        <button type="button" className="btn btn-cancel settings-shortcuts-btn" onClick={onOpenShortcuts}>
          <HelpCircle size={14} aria-hidden="true" /> Keyboard shortcuts
        </button>
      </div>
    </Modal>
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
  themeMode,
  syntaxTheme,
  contextLevel,
  onSelectContext,
  fontSize,
  onChangeFontSize,
  codeFont,
  onSelectCodeFont,
  uiFont,
  onSelectUiFont,
}: {
  metadata: ReviewMetadata;
  diffFiles: ParsedFileDiff[];
  fileChanges: FileChange[];
  onFinish: (data: SummaryData) => void;
  onRefresh: () => void;
  onSelectTheme: (id: ThemeId) => void;
  theme: ThemeId;
  themeMode: 'dark' | 'light';
  syntaxTheme: ReturnType<typeof resolveShikiTheme>;
  contextLevel: ContextLevel;
  onSelectContext: (level: ContextLevel) => void;
  fontSize: number;
  onChangeFontSize: (delta: number) => void;
  codeFont: CodeFontId;
  onSelectCodeFont: (id: CodeFontId) => void;
  uiFont: UiFontId;
  onSelectUiFont: (id: UiFontId) => void;
}): React.JSX.Element {
  const { viewMode, setViewMode, comments, isFileReviewed, markFileReviewed, unmarkFileReviewed } = useReviewStore();
  // Must run inside DiffWorkerPoolProvider: in worker-pool mode pierre reads its
  // highlight theme from the pool's render options, not the per-file `theme`
  // prop. Called from App() (outside the provider) the hook is a no-op and the
  // diff stays on pierre's default theme regardless of the selected one.
  useWorkerPoolThemeSync(syntaxTheme);
  const [viewType, setViewType] = useResponsiveViewType(viewMode as ViewType);
  const quota = useQuotaMonitor();
  const [showHelp, setShowHelp] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [showRefreshWarn, setShowRefreshWarn] = useState(false);
  const [commentIdx, setCommentIdx] = useState(-1);
  const [activeCommentId, setActiveCommentId] = useState<string | null>(null);
  const [scrollDirection, setScrollDirection] = useState<'forward' | 'backward' | null>(null);
  // The diff surface shows one file at a time; this is the selected file.
  const [currentIndex, setCurrentIndex] = useState(0);

  const filePaths = useMemo(() => diffFiles.map((f) => f.newPath || f.oldPath), [diffFiles]);
  // Clamp if the file list shrinks (e.g. after a refresh with fewer files).
  const safeIndex = Math.max(0, Math.min(diffFiles.length - 1, currentIndex));
  const currentPath = filePaths[safeIndex];
  const currentReviewed = currentPath != null && isFileReviewed(currentPath);
  const scrollTop = useCallback(() => {
    document.querySelector('.diff-view-scroll')?.scrollTo({ top: 0 });
  }, []);

  useEffect(() => {
    setViewType(viewMode as ViewType);
  }, [viewMode, setViewType]);

  const handleToggleView = useCallback(() => {
    const next = viewType === 'split' ? 'unified' : 'split';
    setViewType(next);
    setViewMode(next);
  }, [viewType, setViewType, setViewMode]);

  const goToFile = useCallback((index: number) => {
    if (diffFiles.length === 0) return;
    const clamped = Math.max(0, Math.min(diffFiles.length - 1, index));
    setCurrentIndex(clamped);
    scrollTop();
  }, [diffFiles.length, scrollTop]);

  const handleFileClick = useCallback((filePath: string) => {
    const idx = filePaths.indexOf(filePath);
    if (idx >= 0) goToFile(idx);
  }, [filePaths, goToFile]);

  const navigateFile = useCallback((direction: 1 | -1) => {
    goToFile(safeIndex + direction);
  }, [goToFile, safeIndex]);

  const toggleCurrentReviewed = useCallback(() => {
    if (currentPath == null) return;
    if (currentReviewed) {
      unmarkFileReviewed(currentPath);
    } else {
      markFileReviewed(currentPath);
    }
  }, [currentPath, currentReviewed, markFileReviewed, unmarkFileReviewed]);

  // Sorted comments for navigation — matches visual order on page
  const sortedComments = useMemo(() => {
    const fileOrder = new Map<string, number>();
    filePaths.forEach((p, i) => fileOrder.set(p, i));

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
  }, [comments, filePaths]);

  const isFirstComment = commentIdx <= 0;
  const isLastComment = commentIdx >= sortedComments.length - 1;

  const navigateComment = useCallback((direction: 1 | -1) => {
    if (sortedComments.length === 0) return;
    const next = Math.max(0, Math.min(sortedComments.length - 1, commentIdx + direction));
    setCommentIdx(next);
    const c = sortedComments[next];
    setActiveCommentId(c.id);
    setScrollDirection(direction === 1 ? 'forward' : 'backward');
    // Switch to the file that owns the comment; CommentWidget scrolls it into
    // view once its file is shown.
    if (c.type === 'overall') {
      scrollTop();
    } else if (c.filePath) {
      const idx = filePaths.indexOf(c.filePath);
      if (idx >= 0) setCurrentIndex(idx);
    }
  }, [sortedComments, commentIdx, filePaths, scrollTop]);

  const handleAddOverallComment = useCallback(() => {
    // Scroll to top where overall comments section is
    scrollTop();
    // Click the add button if it exists
    const btn = document.querySelector('.overall-comments .btn-add') as HTMLButtonElement | null;
    btn?.click();
  }, [scrollTop]);

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
    <TooltipProvider>
    <div className="app app-with-sidebar">
      {quota.isNearQuota && (
        <div className="quota-warning" role="alert">
          <AlertTriangle size={14} aria-hidden="true" /> localStorage is {quota.usagePercent}% full. Consider discarding old reviews.
        </div>
      )}
      <header className="header">
        <div className="header-row">
          <h1>
            Reviewing <code>{metadata.repoName}</code>:{' '}
            <code>{metadata.baseRef}..{metadata.headRef}</code>
          </h1>
          <div className="toolbar" role="toolbar" aria-label="Review toolbar">
            <div className="toolbar-group" role="group" aria-label="File navigation">
              <span className="toolbar-nav-icon" aria-hidden="true"><FileText size={15} /></span>
              <button className="btn toolbar-btn toolbar-icon-btn" onClick={() => navigateFile(-1)} type="button" title="Previous file (p)" aria-label="Previous file" disabled={safeIndex <= 0}>
                <ChevronLeft size={14} aria-hidden="true" />
              </button>
              <span className="file-position" aria-live="polite" title={currentPath}>
                {diffFiles.length > 0 ? `${safeIndex + 1} / ${diffFiles.length}` : '0 / 0'}
              </span>
              <button className="btn toolbar-btn toolbar-icon-btn" onClick={() => navigateFile(1)} type="button" title="Next file (n)" aria-label="Next file" disabled={safeIndex >= diffFiles.length - 1}>
                <ChevronRight size={14} aria-hidden="true" />
              </button>
              <button
                className={`btn toolbar-btn btn-review-toggle ${currentReviewed ? 'btn-reviewed' : ''}`}
                onClick={toggleCurrentReviewed}
                type="button"
                disabled={currentPath == null}
                title={currentReviewed ? 'Mark file as not reviewed' : 'Mark file as reviewed'}
              >
                {currentReviewed
                  ? <><Check size={14} aria-hidden="true" /> Reviewed</>
                  : <><Circle size={14} aria-hidden="true" /> Review</>}
              </button>
            </div>
            <div className="toolbar-separator" />
            <div className="toolbar-group" role="group" aria-label="Comment navigation">
              <span className="toolbar-nav-icon" aria-hidden="true"><MessageSquare size={15} /></span>
              <button className="btn toolbar-btn toolbar-icon-btn" onClick={() => navigateComment(-1)} type="button" title="Previous comment (k)" aria-label="Previous comment" disabled={sortedComments.length === 0 || isFirstComment}>
                <ChevronLeft size={14} aria-hidden="true" />
              </button>
              <button className="btn toolbar-btn toolbar-icon-btn" onClick={() => navigateComment(1)} type="button" title="Next comment (j)" aria-label="Next comment" disabled={sortedComments.length === 0 || isLastComment}>
                <ChevronRight size={14} aria-hidden="true" />
              </button>
            </div>
            <div className="toolbar-separator" />
            <div className="toolbar-group">
              <button className="btn toolbar-btn" onClick={handleAddOverallComment} type="button" title="Add overall comment (c)">
                <MessageSquarePlus size={14} aria-hidden="true" /> Comment
              </button>
              <button className="btn toolbar-btn" onClick={handleRefresh} type="button" title="Refresh diff">
                <RefreshCw size={14} aria-hidden="true" /> Refresh
              </button>
            </div>
            <div className="toolbar-separator" />
            <div className="toolbar-group">
              <Tooltip label="Settings">
                <button className="btn toolbar-btn toolbar-icon-btn" onClick={() => setShowSettings(true)} type="button" aria-label="Settings">
                  <Settings size={15} aria-hidden="true" />
                </button>
              </Tooltip>
              <DiscardButton />
              <DoneButton metadata={metadata} onFinish={onFinish} />
            </div>
          </div>
        </div>
      </header>
      <div className="app-body">
        <Sidebar files={fileChanges} onFileClick={handleFileClick} activeFile={currentPath} />
        <div className="app-main">
          <OverallComments activeCommentId={activeCommentId} scrollDirection={scrollDirection} />
          <DiffView files={diffFiles} currentIndex={safeIndex} viewType={viewType} themeType={themeMode} syntaxTheme={syntaxTheme} activeCommentId={activeCommentId} scrollDirection={scrollDirection} />
        </div>
      </div>
      <StatusBar totalFiles={fileChanges.length} />
      <SettingsModal
        open={showSettings}
        onOpenChange={setShowSettings}
        viewType={viewType}
        onToggleView={handleToggleView}
        theme={theme}
        onSelectTheme={onSelectTheme}
        contextLevel={contextLevel}
        onSelectContext={onSelectContext}
        fontSize={fontSize}
        onChangeFontSize={onChangeFontSize}
        codeFont={codeFont}
        onSelectCodeFont={onSelectCodeFont}
        uiFont={uiFont}
        onSelectUiFont={onSelectUiFont}
        onOpenShortcuts={() => { setShowSettings(false); setShowHelp(true); }}
      />
      <ShortcutHelpModal open={showHelp} onOpenChange={setShowHelp} />
      <Modal open={showRefreshWarn} onOpenChange={setShowRefreshWarn} ariaLabel="Refresh warning" dialogClassName="modal-dialog">
        <h2>Refresh diff?</h2>
        <p className="modal-detail">
          Line numbers may shift. Comments may become misaligned.
        </p>
        <div className="modal-actions">
          <button className="btn btn-cancel" onClick={() => setShowRefreshWarn(false)} type="button">Cancel</button>
          <button className="btn btn-submit" onClick={confirmRefresh} type="button">Refresh</button>
        </div>
      </Modal>
    </div>
    </TooltipProvider>
  );
}

export function App(): React.JSX.Element {
  const [metadata, setMetadata] = useState<ReviewMetadata | null>(null);
  const [diffFiles, setDiffFiles] = useState<ParsedFileDiff[] | null>(null);
  const [loadState, setLoadState] = useState<LoadState>('loading');
  const [error, setError] = useState('');
  const initialPrefs = useMemo(loadPreferences, []);
  const [theme, setTheme] = useState<ThemeId>(initialPrefs.theme);
  const [fontSize, setFontSize] = useState<number>(initialPrefs.fontSize);
  const [codeFont, setCodeFont] = useState<CodeFontId>(initialPrefs.codeFont);
  const [uiFont, setUiFont] = useState<UiFontId>(initialPrefs.uiFont);
  const [contextLevel, setContextLevel] = useState<ContextLevel>(loadContextLevel);
  const [view, setView] = useState<AppView>('review');
  const [summaryData, setSummaryData] = useState<SummaryData | null>(null);

  const themeMode = THEMES.find((t) => t.id === theme)?.mode ?? 'dark';
  const syntaxTheme = useMemo(() => resolveShikiTheme(theme), [theme]);

  useEffect(() => {
    cleanExpiredSessions();
  }, []);

  useEffect(() => {
    const root = document.documentElement;
    root.setAttribute('data-theme', theme);
    // `--diffs-font-size`/`--diffs-line-height` are what @pierre/diffs reads
    // inside its shadow DOM (custom properties pierce the shadow boundary);
    // line-height tracks pierre's own 13px→20px ratio so rows stay legible.
    root.style.setProperty('--diffs-font-size', `${fontSize}px`);
    root.style.setProperty('--diffs-line-height', `${Math.round(fontSize * (20 / 13))}px`);
    // --font-mono/--font-sans drive the app chrome; --diffs-font-family is the
    // code font inside pierre's shadow DOM (same pierce-the-boundary trick).
    const codeStack = resolveCodeFontStack(codeFont);
    root.style.setProperty('--font-mono', codeStack);
    root.style.setProperty('--diffs-font-family', codeStack);
    root.style.setProperty('--font-sans', resolveUiFontStack(uiFont));
    savePreferences({ theme, fontSize, codeFont, uiFont });
  }, [theme, fontSize, codeFont, uiFont]);

  const fetchDiff = useCallback((level: ContextLevel): Promise<DiffResponse> => {
    return fetch(`/api/v1/diff?context=${level}`).then((r) => {
      if (!r.ok) throw new Error(`diff: ${r.status}`);
      return r.json() as Promise<DiffResponse>;
    });
  }, []);

  const fetchData = useCallback(() => {
    setLoadState('loading');
    setError('');
    Promise.all([
      fetch('/api/v1/metadata').then((r) => {
        if (!r.ok) throw new Error(`metadata: ${r.status}`);
        return r.json() as Promise<ReviewMetadata>;
      }),
      fetchDiff(contextLevel),
    ])
      .then(([meta, diff]) => {
        setMetadata(meta);
        setDiffFiles(sortDiffFiles(diff.files ?? []));
        setLoadState('ready');
      })
      .catch((err: Error) => {
        setError(err.message);
        setLoadState('error');
      });
  }, [fetchDiff, contextLevel]);

  useEffect(() => {
    fetchData();
    // Initial load only; context changes are handled by handleSelectContext so
    // a refetch doesn't reset the whole page to its loading skeleton.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleRefresh = useCallback(() => {
    fetchDiff(contextLevel)
      .then((diff) => {
        setDiffFiles(sortDiffFiles(diff.files ?? []));
      })
      .catch((err: Error) => {
        toast.error('Refresh failed', { description: err.message });
      });
  }, [fetchDiff, contextLevel]);

  const handleSelectContext = useCallback((level: ContextLevel) => {
    setContextLevel(level);
    try {
      localStorage.setItem(CONTEXT_STORAGE_KEY, String(level));
    } catch { /* ignore */ }
    fetchDiff(level)
      .then((diff) => {
        setDiffFiles(sortDiffFiles(diff.files ?? []));
      })
      .catch((err: Error) => {
        toast.error('Failed to change context', { description: err.message });
      });
  }, [fetchDiff]);

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

  const handleChangeFontSize = useCallback((delta: number) => {
    setFontSize((prev) => clampFontSize(prev + delta));
  }, []);

  const handleSelectCodeFont = useCallback((id: CodeFontId) => {
    setCodeFont(id);
  }, []);

  const handleSelectUiFont = useCallback((id: UiFontId) => {
    setUiFont(id);
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
      <Toaster
        position="top-right"
        theme={themeMode}
        richColors
        closeButton
      />
      {view === 'summary' && summaryData ? (
        <SummaryPage
          markdown={summaryData.markdown}
          outputPath={summaryData.outputPath}
          csrfToken={metadata.csrfToken}
          onContinue={handleContinue}
        />
      ) : (
        <DiffWorkerPoolProvider syntaxTheme={syntaxTheme}>
          <AppContent
            metadata={metadata}
            diffFiles={diffFiles}
            fileChanges={metadata.files}
            onFinish={handleFinish}
            onRefresh={handleRefresh}
            onSelectTheme={handleSelectTheme}
            theme={theme}
            themeMode={themeMode}
            syntaxTheme={syntaxTheme}
            contextLevel={contextLevel}
            onSelectContext={handleSelectContext}
            fontSize={fontSize}
            onChangeFontSize={handleChangeFontSize}
            codeFont={codeFont}
            onSelectCodeFont={handleSelectCodeFont}
            uiFont={uiFont}
            onSelectUiFont={handleSelectUiFont}
          />
        </DiffWorkerPoolProvider>
      )}
    </ReviewStoreProvider>
  );
}
