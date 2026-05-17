import { describe, it, expect } from 'vitest';
import { transformFile } from '../transform-diff.js';
import type { ParsedFileDiff, Hunk, Change } from '../../shared/types.js';

function change(type: Change['type'], content: string, old?: number, new_?: number): Change {
  return { type, content, oldLineNumber: old, newLineNumber: new_ };
}

function hunk(overrides: Partial<Hunk> & { changes: Change[] }): Hunk {
  return {
    oldStart: 1,
    oldLines: 1,
    newStart: 1,
    newLines: 1,
    content: '@@ -1,1 +1,1 @@',
    ...overrides,
  };
}

function fileDiff(overrides: Partial<ParsedFileDiff> = {}): ParsedFileDiff {
  return {
    oldPath: 'a.ts',
    newPath: 'a.ts',
    hunks: [],
    status: 'modified',
    additions: 0,
    deletions: 0,
    isBinary: false,
    isLarge: false,
    ...overrides,
  };
}

describe('transformFile', () => {
  it('maps status to react-diff-view DiffType', () => {
    const cases: Array<[ParsedFileDiff['status'], string]> = [
      ['added', 'add'],
      ['deleted', 'delete'],
      ['modified', 'modify'],
      ['renamed', 'rename'],
      ['copied', 'copy'],
    ];
    for (const [status, expected] of cases) {
      const result = transformFile(fileDiff({ status }));
      expect(result.type).toBe(expected);
    }
  });

  it('preserves oldPath and newPath', () => {
    const result = transformFile(fileDiff({ oldPath: 'old.ts', newPath: 'new.ts' }));
    expect(result.oldPath).toBe('old.ts');
    expect(result.newPath).toBe('new.ts');
  });

  it('transforms insert changes', () => {
    const h = hunk({ changes: [change('insert', '+new line', undefined, 5)] });
    const result = transformFile(fileDiff({ hunks: [h] }));
    const c = result.hunks[0].changes[0];
    expect(c).toMatchObject({
      type: 'insert',
      content: '+new line',
      lineNumber: 5,
      isInsert: true,
    });
  });

  it('transforms delete changes', () => {
    const h = hunk({ changes: [change('delete', '-old line', 3)] });
    const result = transformFile(fileDiff({ hunks: [h] }));
    const c = result.hunks[0].changes[0];
    expect(c).toMatchObject({
      type: 'delete',
      content: '-old line',
      lineNumber: 3,
      isDelete: true,
    });
  });

  it('transforms normal changes', () => {
    const h = hunk({ changes: [change('normal', ' same', 2, 2)] });
    const result = transformFile(fileDiff({ hunks: [h] }));
    const c = result.hunks[0].changes[0];
    expect(c).toMatchObject({
      type: 'normal',
      content: ' same',
      oldLineNumber: 2,
      newLineNumber: 2,
      isNormal: true,
    });
  });

  it('preserves hunk metadata', () => {
    const h = hunk({
      oldStart: 10,
      oldLines: 5,
      newStart: 12,
      newLines: 7,
      content: '@@ -10,5 +12,7 @@',
      changes: [],
    });
    const result = transformFile(fileDiff({ hunks: [h] }));
    expect(result.hunks[0]).toMatchObject({
      oldStart: 10,
      oldLines: 5,
      newStart: 12,
      newLines: 7,
      content: '@@ -10,5 +12,7 @@',
    });
  });

  it('handles multiple hunks with multiple changes', () => {
    const h1 = hunk({ changes: [change('normal', ' a', 1, 1), change('insert', '+b', undefined, 2)] });
    const h2 = hunk({ changes: [change('delete', '-c', 5)] });
    const result = transformFile(fileDiff({ hunks: [h1, h2] }));
    expect(result.hunks).toHaveLength(2);
    expect(result.hunks[0].changes).toHaveLength(2);
    expect(result.hunks[1].changes).toHaveLength(1);
  });

  it('defaults missing line numbers to 0', () => {
    const h = hunk({ changes: [change('insert', '+x'), change('delete', '-y')] });
    const result = transformFile(fileDiff({ hunks: [h] }));
    expect(result.hunks[0].changes[0]).toMatchObject({ lineNumber: 0 });
    expect(result.hunks[0].changes[1]).toMatchObject({ lineNumber: 0 });
  });

  it('sets ending newline and mode fields', () => {
    const result = transformFile(fileDiff());
    expect(result.oldEndingNewLine).toBe(true);
    expect(result.newEndingNewLine).toBe(true);
    expect(result.oldMode).toBe('');
    expect(result.newMode).toBe('');
    expect(result.oldRevision).toBe('');
    expect(result.newRevision).toBe('');
  });
});
