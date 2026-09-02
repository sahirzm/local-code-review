import type { ParsedFileDiff, Comment, OrphanReason } from '../shared/types.js';

/**
 * Line/range comments are pinned to the verbatim content of their anchor
 * line(s). When the diff is recomputed (a context change, a refresh after the
 * agent edits code), a pinned comment whose line drifted can no longer be
 * trusted to point at the right place. This module captures the pin at
 * creation time and reconciles pins against a fresh diff, moving stale ones
 * into an `orphaned` state the UI surfaces for re-pin or delete.
 */

/** Look up the verbatim content of a single (side, line) in a parsed diff. */
export function lineContentAt(
  file: ParsedFileDiff,
  side: 'old' | 'new',
  line: number,
): string | null {
  for (const hunk of file.hunks) {
    for (const change of hunk.changes) {
      const num = side === 'old' ? change.oldLineNumber : change.newLineNumber;
      if (num === line) return change.content;
    }
  }
  return null;
}

/** Join the content of a comment's pinned line range into a snippet. */
export function snippetForRange(
  file: ParsedFileDiff,
  side: 'old' | 'new',
  startLine: number,
  endLine: number,
): string {
  const lines: string[] = [];
  for (let l = startLine; l <= endLine; l++) {
    const content = lineContentAt(file, side, l);
    if (content != null) lines.push(content);
  }
  return lines.join('\n');
}

/** Find the file diff whose new or old path matches the comment. */
function fileForComment(
  files: ParsedFileDiff[],
  filePath: string,
): ParsedFileDiff | undefined {
  return files.find((f) => f.newPath === filePath || f.oldPath === filePath);
}

export interface PinnedFields {
  pinSnippet: string;
  pinLineCount: number;
}

/**
 * Compute the pin fields for a newly created line/range comment. Returns null
 * when the anchor content can't be read (caller then leaves the comment
 * unpinned, which simply opts it out of orphan detection).
 */
export function computePin(
  files: ParsedFileDiff[],
  filePath: string,
  side: 'old' | 'new',
  startLine: number,
  endLine: number,
): PinnedFields | null {
  const file = fileForComment(files, filePath);
  if (!file) return null;
  const snippet = snippetForRange(file, side, startLine, endLine);
  if (snippet === '') return null;
  return { pinSnippet: snippet, pinLineCount: endLine - startLine + 1 };
}

/**
 * Reconcile one pinned comment against a fresh diff. Returns the (possibly
 * mutated) comment: unchanged when its pin still matches, orphaned with a
 * reason when it drifted, or reopened when a previously orphaned comment
 * matches again. Comments without a pin, and file/overall comments, pass
 * through untouched. Resolving a comment is immune to orphaning.
 */
export function reconcileComment(comment: Comment, files: ParsedFileDiff[]): Comment {
  if (comment.type !== 'line' && comment.type !== 'range') return comment;
  if (comment.pinSnippet == null || comment.startLine == null || comment.side == null) {
    return comment;
  }
  if (comment.status === 'resolved') return comment;

  const side = comment.side;
  const startLine = comment.startLine;
  const endLine = comment.endLine ?? comment.startLine;

  let reason: OrphanReason | null = null;
  const file = fileForComment(files, comment.filePath ?? '');
  if (!file) {
    reason = 'file_missing';
  } else {
    const current = snippetForRange(file, side, startLine, endLine);
    if (current === '') {
      reason = 'line_out_of_range';
    } else if (current !== comment.pinSnippet) {
      reason = 'snippet_mismatch';
    }
  }

  if (reason == null) {
    // Pin still valid. Recover a previously orphaned comment.
    if (comment.status === 'orphaned') {
      const { orphanReason: _drop, ...rest } = comment;
      return { ...rest, status: 'open' };
    }
    return comment;
  }

  if (comment.status === 'orphaned' && comment.orphanReason === reason) return comment;
  return { ...comment, status: 'orphaned', orphanReason: reason };
}

/** Reconcile every comment against a fresh diff. */
export function reconcileComments(comments: Comment[], files: ParsedFileDiff[]): Comment[] {
  return comments.map((c) => reconcileComment(c, files));
}

/**
 * Attempt to re-pin an orphaned comment against a fresh diff by searching for
 * its snippet on the same side. Returns the re-pinned comment (status open,
 * lines updated) when a unique match is found, or null when it can't be
 * re-pinned automatically.
 */
export function repinComment(comment: Comment, files: ParsedFileDiff[]): Comment | null {
  if (comment.pinSnippet == null || comment.side == null || comment.filePath == null) return null;
  const file = fileForComment(files, comment.filePath);
  if (!file) return null;
  const side = comment.side;
  const snippetLines = comment.pinSnippet.split('\n');
  const count = snippetLines.length;

  // Collect candidate start lines on the target side, in order.
  const lineNumbers: number[] = [];
  for (const hunk of file.hunks) {
    for (const change of hunk.changes) {
      const num = side === 'old' ? change.oldLineNumber : change.newLineNumber;
      if (num != null) lineNumbers.push(num);
    }
  }
  lineNumbers.sort((a, b) => a - b);

  const matches: number[] = [];
  for (const start of lineNumbers) {
    const candidate = snippetForRange(file, side, start, start + count - 1);
    if (candidate === comment.pinSnippet) matches.push(start);
  }
  if (matches.length !== 1) return null; // ambiguous or no match

  const start = matches[0];
  const { orphanReason: _drop, ...rest } = comment;
  return { ...rest, status: 'open', startLine: start, endLine: start + count - 1 };
}
