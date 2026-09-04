import { describe, it, expect } from 'vitest';
import { sortDiffFiles } from '../App.js';
import type { ParsedFileDiff } from '../shared/types.js';

function file(path: string, oldPath?: string): ParsedFileDiff {
  return {
    oldPath: oldPath ?? path,
    newPath: path,
    hunks: [],
    status: 'modified',
    additions: 0,
    deletions: 0,
    isBinary: false,
    isLarge: false,
    rawPatch: '',
  };
}

describe('sortDiffFiles', () => {
  it('matches sidebar tree order with directories before files', () => {
    const sorted = sortDiffFiles([
      file('README.md'),
      file('src/nested/b.ts'),
      file('src/z.ts'),
      file('src/nested/a.ts'),
      file('src/a.ts'),
    ]);
    expect(sorted.map((f) => f.newPath)).toEqual([
      'src/nested/a.ts',
      'src/nested/b.ts',
      'src/a.ts',
      'src/z.ts',
      'README.md',
    ]);
  });

  it('sorts by newPath for renamed files (its display path)', () => {
    const sorted = sortDiffFiles([file('b.ts', 'old-b.ts'), file('a.ts', 'old-a.ts')]);
    expect(sorted.map((f) => f.newPath)).toEqual(['a.ts', 'b.ts']);
  });

  it('does not mutate the input array', () => {
    const input = [file('b.ts'), file('a.ts')];
    const sorted = sortDiffFiles(input);
    expect(input.map((f) => f.newPath)).toEqual(['b.ts', 'a.ts']);
    expect(sorted).not.toBe(input);
  });
});
