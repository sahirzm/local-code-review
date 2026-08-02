import { getSingularPatch } from '@pierre/diffs';
import type { FileDiffMetadata, DiffLineAnnotation, AnnotationSide, SelectedLineRange } from '@pierre/diffs';
import type { ParsedFileDiff, Comment } from '../../shared/types.js';

/**
 * @pierre/diffs annotations are keyed by `additions`/`deletions`; our comment
 * model uses `new`/`old`. These two helpers are the single crossing point
 * between the two vocabularies.
 */
export function sideToAnnotation(side: 'old' | 'new'): AnnotationSide {
  return side === 'old' ? 'deletions' : 'additions';
}

export function annotationToSide(side: AnnotationSide): 'old' | 'new' {
  return side === 'deletions' ? 'old' : 'new';
}

export function parsePatch(file: ParsedFileDiff): FileDiffMetadata | null {
  if (!file.rawPatch) return null;
  try {
    return getSingularPatch(file.rawPatch);
  } catch {
    // A patch that doesn't parse to exactly one file (or is malformed) falls
    // back to a non-highlighted plain view handled by the caller.
    return null;
  }
}

/** Metadata carried on each pierre line annotation so renderAnnotation can
 * recover the comments (and any open form) for that line. */
export interface DiffAnnotationMetadata {
  comments: Comment[];
  side: 'old' | 'new';
  lineNumber: number;
}

/**
 * Projects line/range comments onto pierre line annotations. Range comments
 * anchor to their end line (matching the previous react-diff-view behavior),
 * and multiple comments on one line collapse into a single annotation so
 * renderAnnotation emits one stacked widget container per line.
 */
export function commentsToAnnotations(
  comments: Comment[],
): DiffLineAnnotation<DiffAnnotationMetadata>[] {
  const byLine = new Map<string, DiffLineAnnotation<DiffAnnotationMetadata>>();
  for (const comment of comments) {
    if (comment.type !== 'line' && comment.type !== 'range') continue;
    if (comment.startLine == null || comment.side == null) continue;
    const side = comment.side === 'old' ? 'old' : 'new';
    const lineNumber = comment.type === 'range' ? (comment.endLine ?? comment.startLine) : comment.startLine;
    const key = `${side}:${lineNumber}`;
    const existing = byLine.get(key);
    if (existing) {
      existing.metadata.comments.push(comment);
      continue;
    }
    byLine.set(key, {
      side: sideToAnnotation(side),
      lineNumber,
      metadata: { comments: [comment], side, lineNumber },
    });
  }
  return Array.from(byLine.values());
}

/** Normalizes a pierre selection range into our line/side comment anchor. */
export function selectionToAnchor(range: SelectedLineRange): {
  line: number;
  endLine: number;
  side: 'old' | 'new';
} {
  const side = annotationToSide(range.side ?? 'additions');
  const lo = Math.min(range.start, range.end);
  const hi = Math.max(range.start, range.end);
  return { line: lo, endLine: hi, side };
}
