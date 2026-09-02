import { describe, it, expect } from 'vitest';
import type { ParsedFileDiff, Comment } from '../../shared/types.js';
import { computePin, reconcileComment, repinComment, lineContentAt } from '../pin.js';

function file(overrides: Partial<ParsedFileDiff> = {}): ParsedFileDiff {
  return {
    oldPath: 'a.ts',
    newPath: 'a.ts',
    status: 'modified',
    additions: 0,
    deletions: 0,
    isBinary: false,
    isLarge: false,
    rawPatch: '',
    hunks: [
      {
        oldStart: 1,
        oldLines: 3,
        newStart: 1,
        newLines: 3,
        content: '',
        changes: [
          { type: 'normal', oldLineNumber: 1, newLineNumber: 1, content: 'const a = 1;' },
          { type: 'insert', newLineNumber: 2, content: 'const b = 2;' },
          { type: 'normal', oldLineNumber: 2, newLineNumber: 3, content: 'return a + b;' },
        ],
      },
    ],
    ...overrides,
  };
}

function comment(overrides: Partial<Comment> = {}): Comment {
  return {
    id: 'c1',
    type: 'line',
    category: 'fix',
    text: 'note',
    filePath: 'a.ts',
    startLine: 2,
    endLine: 2,
    side: 'new',
    status: 'open',
    createdAt: 't',
    updatedAt: 't',
    ...overrides,
  };
}

describe('pin utilities', () => {
  it('reads verbatim line content by side/line', () => {
    expect(lineContentAt(file(), 'new', 2)).toBe('const b = 2;');
    expect(lineContentAt(file(), 'new', 99)).toBeNull();
  });

  it('computes a pin snippet for a single line', () => {
    const pin = computePin([file()], 'a.ts', 'new', 2, 2);
    expect(pin).toEqual({ pinSnippet: 'const b = 2;', pinLineCount: 1 });
  });

  it('computes a multi-line pin snippet for a range', () => {
    const pin = computePin([file()], 'a.ts', 'new', 2, 3);
    expect(pin).toEqual({ pinSnippet: 'const b = 2;\nreturn a + b;', pinLineCount: 2 });
  });

  it('leaves a matching pin untouched', () => {
    const c = comment({ pinSnippet: 'const b = 2;', pinLineCount: 1 });
    expect(reconcileComment(c, [file()])).toBe(c);
  });

  it('orphans a comment whose line content changed', () => {
    const c = comment({ pinSnippet: 'const OLD = 0;', pinLineCount: 1 });
    const out = reconcileComment(c, [file()]);
    expect(out.status).toBe('orphaned');
    expect(out.orphanReason).toBe('snippet_mismatch');
  });

  it('orphans a comment whose file disappeared', () => {
    const c = comment({ pinSnippet: 'const b = 2;', pinLineCount: 1, filePath: 'gone.ts' });
    const out = reconcileComment(c, [file()]);
    expect(out.status).toBe('orphaned');
    expect(out.orphanReason).toBe('file_missing');
  });

  it('never orphans a resolved comment', () => {
    const c = comment({ pinSnippet: 'const OLD = 0;', pinLineCount: 1, status: 'resolved' });
    expect(reconcileComment(c, [file()])).toBe(c);
  });

  it('recovers a previously orphaned comment when the pin matches again', () => {
    const c = comment({ pinSnippet: 'const b = 2;', pinLineCount: 1, status: 'orphaned', orphanReason: 'snippet_mismatch' });
    const out = reconcileComment(c, [file()]);
    expect(out.status).toBe('open');
    expect(out.orphanReason).toBeUndefined();
  });

  it('re-pins an orphaned comment to a moved line with a unique match', () => {
    const moved = file({
      hunks: [
        {
          oldStart: 1,
          oldLines: 4,
          newStart: 1,
          newLines: 4,
          content: '',
          changes: [
            { type: 'insert', newLineNumber: 1, content: '// header' },
            { type: 'normal', oldLineNumber: 1, newLineNumber: 2, content: 'const a = 1;' },
            { type: 'insert', newLineNumber: 3, content: 'const b = 2;' },
            { type: 'normal', oldLineNumber: 2, newLineNumber: 4, content: 'return a + b;' },
          ],
        },
      ],
    });
    const c = comment({ pinSnippet: 'const b = 2;', pinLineCount: 1, status: 'orphaned', orphanReason: 'snippet_mismatch', startLine: 2, endLine: 2 });
    const out = repinComment(c, [moved]);
    expect(out).not.toBeNull();
    expect(out?.startLine).toBe(3);
    expect(out?.status).toBe('open');
  });

  it('refuses to re-pin when the snippet is ambiguous', () => {
    const dup = file({
      hunks: [
        {
          oldStart: 1,
          oldLines: 2,
          newStart: 1,
          newLines: 2,
          content: '',
          changes: [
            { type: 'insert', newLineNumber: 1, content: 'dupe' },
            { type: 'insert', newLineNumber: 2, content: 'dupe' },
          ],
        },
      ],
    });
    const c = comment({ pinSnippet: 'dupe', pinLineCount: 1, status: 'orphaned' });
    expect(repinComment(c, [dup])).toBeNull();
  });
});
