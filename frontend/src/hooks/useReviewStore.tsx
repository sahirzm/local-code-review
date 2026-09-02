import { createContext, useContext, useReducer, useCallback, useMemo, useEffect, useRef, type ReactNode } from 'react';
import type { Comment, ReviewSession, ReviewMetadata, ParsedFileDiff } from '../../../shared/types.js';
import { saveSession, loadSession, clearSession, getSessionKey, hashRepoPath } from './useSession.js';
import { reconcileComments, repinComment as repinCommentFn } from '../utils/pin.js';

interface ReviewState {
  comments: Comment[];
  viewMode: 'split' | 'unified';
  reviewedFiles: string[];
}

type Action =
  | { type: 'ADD'; comment: Comment }
  | { type: 'EDIT'; id: string; updates: { text?: string; category?: Comment['category'] } }
  | { type: 'SET_STATUS'; id: string; status: NonNullable<Comment['status']> }
  | { type: 'REPLACE_ALL'; comments: Comment[] }
  | { type: 'DELETE'; id: string }
  | { type: 'SET_VIEW_MODE'; viewMode: 'split' | 'unified' }
  | { type: 'MARK_REVIEWED'; filePath: string }
  | { type: 'UNMARK_REVIEWED'; filePath: string }
  | { type: 'RESTORE'; state: ReviewState }
  | { type: 'DISCARD' };

export interface ReviewStore {
  comments: Comment[];
  viewMode: 'split' | 'unified';
  reviewedFiles: string[];
  addComment: (comment: Omit<Comment, 'id' | 'createdAt' | 'updatedAt'>) => void;
  editComment: (id: string, updates: { text?: string; category?: Comment['category'] }) => void;
  setCommentStatus: (id: string, status: NonNullable<Comment['status']>) => void;
  deleteComment: (id: string) => void;
  /** Reconcile every pinned comment against a freshly-fetched diff. */
  reconcileAgainstDiff: (files: ParsedFileDiff[]) => void;
  /** Try to re-pin an orphaned comment against the current diff. Returns whether it succeeded. */
  repinComment: (id: string, files: ParsedFileDiff[]) => boolean;
  getCommentsForFile: (filePath: string) => Comment[];
  getCommentsForLine: (filePath: string, line: number, side: 'old' | 'new') => Comment[];
  getAllComments: () => Comment[];
  setViewMode: (mode: 'split' | 'unified') => void;
  discardReview: () => void;
  markFileReviewed: (filePath: string) => void;
  unmarkFileReviewed: (filePath: string) => void;
  isFileReviewed: (filePath: string) => boolean;
}

const INITIAL_STATE: ReviewState = { comments: [], viewMode: 'split', reviewedFiles: [] };

function reducer(state: ReviewState, action: Action): ReviewState {
  switch (action.type) {
    case 'ADD':
      return { ...state, comments: [...state.comments, action.comment] };
    case 'EDIT':
      return {
        ...state,
        comments: state.comments.map((c) =>
          c.id === action.id
            ? { ...c, ...action.updates, updatedAt: new Date().toISOString() }
            : c,
        ),
      };
    case 'REPLACE_ALL':
      return { ...state, comments: action.comments };
    case 'DELETE':
      return { ...state, comments: state.comments.filter((c) => c.id !== action.id) };
    case 'SET_STATUS':
      return {
        ...state,
        comments: state.comments.map((c) =>
          c.id === action.id
            ? { ...c, status: action.status, updatedAt: new Date().toISOString() }
            : c,
        ),
      };
    case 'SET_VIEW_MODE':
      return { ...state, viewMode: action.viewMode };
    case 'MARK_REVIEWED':
      return state.reviewedFiles.includes(action.filePath)
        ? state
        : { ...state, reviewedFiles: [...state.reviewedFiles, action.filePath] };
    case 'UNMARK_REVIEWED':
      return { ...state, reviewedFiles: state.reviewedFiles.filter((f) => f !== action.filePath) };
    case 'RESTORE':
      return action.state;
    case 'DISCARD':
      return { ...INITIAL_STATE };
  }
}

const ReviewContext = createContext<ReviewStore | null>(null);

interface ProviderProps {
  children: ReactNode;
  metadata?: ReviewMetadata | null;
}

export function ReviewStoreProvider({ children, metadata }: ProviderProps): React.JSX.Element {
  const [state, dispatch] = useReducer(reducer, INITIAL_STATE);
  const sessionKeyRef = useRef<string | null>(null);
  const metadataRef = useRef(metadata);
  metadataRef.current = metadata;
  const restoredRef = useRef(false);

  // Compute session key when metadata is available
  useEffect(() => {
    if (!metadata) return;
    const hash = hashRepoPath(metadata.commitRange.split('..')[0] ?? metadata.commitRange);
    sessionKeyRef.current = getSessionKey(hash, metadata.commitRange);

    if (restoredRef.current) return;
    restoredRef.current = true;

    const saved = loadSession(sessionKeyRef.current);
    if (saved && saved.comments.length > 0) {
      dispatch({
        type: 'RESTORE',
        state: {
          comments: saved.comments,
          viewMode: saved.viewMode ?? 'split',
          reviewedFiles: saved.reviewedFiles ?? [],
        },
      });
    }
  }, [metadata]);

  // Persist to localStorage on state change
  const stateRef = useRef(state);
  stateRef.current = state;

  useEffect(() => {
    if (!sessionKeyRef.current || !metadataRef.current || !restoredRef.current) return;
    const now = new Date().toISOString();
    const session: ReviewSession = {
      version: 2,
      commitRange: metadataRef.current.commitRange,
      repoPath: metadataRef.current.repoName,
      repoPathHash: hashRepoPath(metadataRef.current.commitRange.split('..')[0] ?? metadataRef.current.commitRange),
      comments: state.comments,
      reviewedFiles: state.reviewedFiles,
      viewMode: state.viewMode,
      createdAt: now,
      lastAccessedAt: now,
    };
    saveSession(sessionKeyRef.current, session);
  }, [state.comments, state.viewMode, state.reviewedFiles]);

  // Server backup every 30s
  useEffect(() => {
    if (!metadata) return;
    const interval = setInterval(() => {
      if (!sessionKeyRef.current || !metadataRef.current) return;
      const current = stateRef.current;
      const now = new Date().toISOString();
      const session: ReviewSession = {
        version: 2,
        commitRange: metadataRef.current.commitRange,
        repoPath: metadataRef.current.repoName,
        repoPathHash: hashRepoPath(metadataRef.current.commitRange.split('..')[0] ?? metadataRef.current.commitRange),
        comments: current.comments,
        reviewedFiles: current.reviewedFiles,
        viewMode: current.viewMode,
        createdAt: now,
        lastAccessedAt: now,
      };
      fetch('/api/v1/save-session', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-CSRF-Token': metadataRef.current.csrfToken,
        },
        body: JSON.stringify({ session, _csrf: metadataRef.current.csrfToken }),
      }).catch((err: unknown) => {
        console.error('Session backup failed:', err);
      });
    }, 30_000);
    return () => clearInterval(interval);
  }, [metadata]);

  const addComment = useCallback(
    (comment: Omit<Comment, 'id' | 'createdAt' | 'updatedAt'>) => {
      const now = new Date().toISOString();
      dispatch({
        type: 'ADD',
        comment: { status: 'open', ...comment, id: crypto.randomUUID(), createdAt: now, updatedAt: now },
      });
    },
    [],
  );

  const editComment = useCallback(
    (id: string, updates: { text?: string; category?: Comment['category'] }) => {
      dispatch({ type: 'EDIT', id, updates });
    },
    [],
  );

  const deleteComment = useCallback((id: string) => {
    dispatch({ type: 'DELETE', id });
  }, []);

  const setCommentStatus = useCallback(
    (id: string, status: NonNullable<Comment['status']>) => {
      dispatch({ type: 'SET_STATUS', id, status });
    },
    [],
  );

  const reconcileAgainstDiff = useCallback((files: ParsedFileDiff[]) => {
    const next = reconcileComments(stateRef.current.comments, files);
    // Only dispatch when something actually changed to avoid a redundant render.
    const changed = next.some((c, i) => c !== stateRef.current.comments[i]);
    if (changed) dispatch({ type: 'REPLACE_ALL', comments: next });
  }, []);

  const repinComment = useCallback((id: string, files: ParsedFileDiff[]): boolean => {
    const target = stateRef.current.comments.find((c) => c.id === id);
    if (!target) return false;
    const repinned = repinCommentFn(target, files);
    if (!repinned) return false;
    dispatch({
      type: 'REPLACE_ALL',
      comments: stateRef.current.comments.map((c) => (c.id === id ? repinned : c)),
    });
    return true;
  }, []);

  const setViewMode = useCallback((mode: 'split' | 'unified') => {
    dispatch({ type: 'SET_VIEW_MODE', viewMode: mode });
  }, []);

  const discardReview = useCallback(() => {
    if (sessionKeyRef.current) {
      clearSession(sessionKeyRef.current);
    }
    dispatch({ type: 'DISCARD' });
  }, []);

  const markFileReviewed = useCallback((filePath: string) => {
    dispatch({ type: 'MARK_REVIEWED', filePath });
  }, []);

  const unmarkFileReviewed = useCallback((filePath: string) => {
    dispatch({ type: 'UNMARK_REVIEWED', filePath });
  }, []);

  const isFileReviewed = useCallback(
    (filePath: string) => state.reviewedFiles.includes(filePath),
    [state.reviewedFiles],
  );

  const getCommentsForFile = useCallback(
    (filePath: string) => state.comments.filter((c) => c.filePath === filePath),
    [state.comments],
  );

  const getCommentsForLine = useCallback(
    (filePath: string, line: number, side: 'old' | 'new') =>
      state.comments.filter(
        (c) =>
          c.filePath === filePath &&
          c.side === side &&
          c.startLine !== undefined &&
          c.startLine <= line &&
          (c.endLine ?? c.startLine) >= line,
      ),
    [state.comments],
  );

  const getAllComments = useCallback(() => state.comments, [state.comments]);

  const value = useMemo(
    () => ({
      comments: state.comments,
      viewMode: state.viewMode,
      reviewedFiles: state.reviewedFiles,
      addComment,
      editComment,
      setCommentStatus,
      deleteComment,
      reconcileAgainstDiff,
      repinComment,
      getCommentsForFile,
      getCommentsForLine,
      getAllComments,
      setViewMode,
      discardReview,
      markFileReviewed,
      unmarkFileReviewed,
      isFileReviewed,
    }),
    [state.comments, state.viewMode, state.reviewedFiles, addComment, editComment, setCommentStatus, deleteComment, reconcileAgainstDiff, repinComment, getCommentsForFile, getCommentsForLine, getAllComments, setViewMode, discardReview, markFileReviewed, unmarkFileReviewed, isFileReviewed],
  );

  return <ReviewContext.Provider value={value}>{children}</ReviewContext.Provider>;
}

export function useReviewStore(): ReviewStore {
  const ctx = useContext(ReviewContext);
  if (!ctx) throw new Error('useReviewStore must be used within ReviewStoreProvider');
  return ctx;
}
