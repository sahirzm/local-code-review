import { describe, it, expect } from 'vitest';
import {
  sideToAnnotation,
  annotationToSide,
  parsePatch,
  commentsToAnnotations,
  selectionToAnchor,
} from '../adapter.js';
import type { ParsedFileDiff, Comment } from '../../../shared/types.js';

function comment(overrides: Partial<Comment>): Comment {
  return {
    id: overrides.id ?? 'c1',
    type: overrides.type ?? 'line',
    category: 'question',
    text: 'x',
    filePath: 'a.ts',
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
    ...overrides,
  };
}

const PATCH = [
  'diff --git a/a.ts b/a.ts',
  '--- a/a.ts',
  '+++ b/a.ts',
  '@@ -1,2 +1,2 @@',
  ' a',
  '-old',
  '+new',
  '',
].join('\n');

describe('side vocabulary mapping', () => {
  it('maps new↔additions and old↔deletions round-trip', () => {
    expect(sideToAnnotation('new')).toBe('additions');
    expect(sideToAnnotation('old')).toBe('deletions');
    expect(annotationToSide('additions')).toBe('new');
    expect(annotationToSide('deletions')).toBe('old');
  });
});

describe('parsePatch', () => {
  it('parses a valid single-file patch into pierre metadata', () => {
    const file: ParsedFileDiff = {
      oldPath: 'a.ts', newPath: 'a.ts', hunks: [], status: 'modified',
      additions: 1, deletions: 1, isBinary: false, isLarge: false, rawPatch: PATCH,
    };
    const result = parsePatch(file);
    expect(result).not.toBeNull();
    expect(result?.name).toBe('a.ts');
    expect(result?.hunks.length).toBeGreaterThan(0);
  });

  it('returns null for an empty rawPatch', () => {
    const file: ParsedFileDiff = {
      oldPath: 'a.ts', newPath: 'a.ts', hunks: [], status: 'modified',
      additions: 0, deletions: 0, isBinary: false, isLarge: false, rawPatch: '',
    };
    expect(parsePatch(file)).toBeNull();
  });

  it('returns null for a malformed patch instead of throwing', () => {
    const file: ParsedFileDiff = {
      oldPath: 'a.ts', newPath: 'a.ts', hunks: [], status: 'modified',
      additions: 0, deletions: 0, isBinary: false, isLarge: false, rawPatch: 'not a patch',
    };
    expect(parsePatch(file)).toBeNull();
  });
});

describe('commentsToAnnotations', () => {
  it('anchors a line comment to its start line on the correct side', () => {
    const result = commentsToAnnotations([comment({ type: 'line', startLine: 5, side: 'new' })]);
    expect(result).toHaveLength(1);
    expect(result[0].side).toBe('additions');
    expect(result[0].lineNumber).toBe(5);
    expect(result[0].metadata.comments).toHaveLength(1);
  });

  it('anchors a range comment to its end line', () => {
    const result = commentsToAnnotations([comment({ type: 'range', startLine: 5, endLine: 9, side: 'old' })]);
    expect(result).toHaveLength(1);
    expect(result[0].side).toBe('deletions');
    expect(result[0].lineNumber).toBe(9);
  });

  it('collapses multiple comments on the same line+side into one annotation', () => {
    const result = commentsToAnnotations([
      comment({ id: 'c1', startLine: 5, side: 'new' }),
      comment({ id: 'c2', startLine: 5, side: 'new' }),
    ]);
    expect(result).toHaveLength(1);
    expect(result[0].metadata.comments.map((c) => c.id)).toEqual(['c1', 'c2']);
  });

  it('keeps same-line comments on opposite sides separate', () => {
    const result = commentsToAnnotations([
      comment({ id: 'c1', startLine: 5, side: 'new' }),
      comment({ id: 'c2', startLine: 5, side: 'old' }),
    ]);
    expect(result).toHaveLength(2);
  });

  it('ignores file and overall comments', () => {
    const result = commentsToAnnotations([
      comment({ id: 'c1', type: 'file' }),
      comment({ id: 'c2', type: 'overall' }),
    ]);
    expect(result).toHaveLength(0);
  });
});

describe('selectionToAnchor', () => {
  it('normalizes an inverted selection and maps the side', () => {
    const anchor = selectionToAnchor({ start: 9, end: 4, side: 'deletions' });
    expect(anchor).toEqual({ line: 4, endLine: 9, side: 'old' });
  });

  it('defaults to the additions side when unspecified', () => {
    const anchor = selectionToAnchor({ start: 3, end: 3 });
    expect(anchor).toEqual({ line: 3, endLine: 3, side: 'new' });
  });
});
