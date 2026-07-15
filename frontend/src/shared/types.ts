export interface Comment {
  id: string;
  type: 'line' | 'range' | 'file' | 'overall';
  category: 'fix' | 'question' | 'suggestion' | 'nit';
  text: string;
  filePath?: string;
  startLine?: number;
  endLine?: number;
  side?: 'old' | 'new';
  createdAt: string;
  updatedAt: string;
}

export interface CLIOptions {
  port: number;
  base?: string;
  noOpen: boolean;
  output?: string;
  commits?: [string, string];
  staged: boolean;
  unstaged: boolean;
  working: boolean;
  fetch: boolean;
}

export interface FileChange {
  path: string;
  oldPath?: string;
  status: 'added' | 'modified' | 'deleted' | 'renamed' | 'copied';
  additions: number;
  deletions: number;
}

export interface ReviewMetadata {
  repoName: string;
  commitRange: string;
  baseRef: string;
  headRef: string;
  files: FileChange[];
  timestamp: string;
  csrfToken: string;
}

export interface DiffResponse {
  files: ParsedFileDiff[];
}

export interface ParsedFileDiff {
  oldPath: string;
  newPath: string;
  hunks: Hunk[];
  status: FileChange['status'];
  additions: number;
  deletions: number;
  isBinary: boolean;
  isLarge: boolean;
}

export interface Hunk {
  oldStart: number;
  oldLines: number;
  newStart: number;
  newLines: number;
  content: string;
  changes: Change[];
}

export interface Change {
  type: 'insert' | 'delete' | 'normal';
  oldLineNumber?: number;
  newLineNumber?: number;
  content: string;
}

export interface FinishRequest {
  comments: Comment[];
  reviewedFiles: string[];
  metadata: {
    commitRange: string;
    timestamp: string;
  };
  _csrf: string;
}

export interface FinishResponse {
  success: boolean;
  outputPath: string;
  markdown: string;
}

export interface SessionBackup {
  session: ReviewSession;
  _csrf: string;
}

export interface ReviewSession {
  version: 2;
  commitRange: string;
  repoPath: string;
  repoPathHash: string;
  comments: Comment[];
  reviewedFiles: string[];
  viewMode: 'split' | 'unified';
  createdAt: string;
  lastAccessedAt: string;
}

export type ThemeId =
  | 'default-dark'
  | 'default-light'
  | 'catppuccin-latte'
  | 'catppuccin-frappe'
  | 'catppuccin-macchiato'
  | 'catppuccin-mocha';

export interface UserPreferences {
  theme: ThemeId;
  /** Diff text size in px. */
  fontSize: number;
}

export interface FileTreeNode {
  name: string;
  path: string;
  type: 'file' | 'directory';
  children?: FileTreeNode[];
  status?: FileChange['status'];
  additions?: number;
  deletions?: number;
  isReviewed: boolean;
  commentCount: number;
}
