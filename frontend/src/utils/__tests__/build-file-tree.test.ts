import { describe, it, expect } from 'vitest';
import { buildFileTree } from '../build-file-tree.js';
import type { FileChange } from '../../shared/types.js';

function file(path: string, status: FileChange['status'] = 'modified'): FileChange {
  return { path, status, additions: 1, deletions: 0 };
}

describe('buildFileTree', () => {
  it('returns empty array for no files', () => {
    expect(buildFileTree([], [], new Map())).toEqual([]);
  });

  it('creates flat file nodes for root-level files', () => {
    const tree = buildFileTree([file('README.md'), file('index.ts')], [], new Map());
    expect(tree).toHaveLength(2);
    expect(tree[0]).toMatchObject({ name: 'index.ts', type: 'file', path: 'index.ts' });
    expect(tree[1]).toMatchObject({ name: 'README.md', type: 'file', path: 'README.md' });
  });

  it('nests files under directory nodes', () => {
    const tree = buildFileTree([file('src/utils/helpers.ts')], [], new Map());
    expect(tree).toHaveLength(1);
    expect(tree[0]).toMatchObject({ name: 'src', type: 'directory' });
    expect(tree[0].children).toHaveLength(1);
    expect(tree[0].children![0]).toMatchObject({ name: 'utils', type: 'directory' });
    expect(tree[0].children![0].children![0]).toMatchObject({
      name: 'helpers.ts',
      type: 'file',
      path: 'src/utils/helpers.ts',
    });
  });

  it('sorts directories before files, alphabetically within each group', () => {
    const tree = buildFileTree(
      [file('z-file.ts'), file('a-dir/inner.ts'), file('a-file.ts')],
      [],
      new Map(),
    );
    expect(tree.map((n) => n.name)).toEqual(['a-dir', 'a-file.ts', 'z-file.ts']);
  });

  it('marks reviewed files', () => {
    const tree = buildFileTree(
      [file('a.ts'), file('b.ts')],
      ['a.ts'],
      new Map(),
    );
    const a = tree.find((n) => n.name === 'a.ts')!;
    const b = tree.find((n) => n.name === 'b.ts')!;
    expect(a.isReviewed).toBe(true);
    expect(b.isReviewed).toBe(false);
  });

  it('attaches comment counts to file nodes', () => {
    const counts = new Map([['src/index.ts', 3]]);
    const tree = buildFileTree([file('src/index.ts')], [], counts);
    const leaf = tree[0].children![0];
    expect(leaf.commentCount).toBe(3);
  });

  it('defaults commentCount to 0 when file has no comments', () => {
    const tree = buildFileTree([file('foo.ts')], [], new Map());
    expect(tree[0].commentCount).toBe(0);
  });

  it('preserves file status on leaf nodes', () => {
    const tree = buildFileTree([file('added.ts', 'added'), file('del.ts', 'deleted')], [], new Map());
    expect(tree[0]).toMatchObject({ name: 'added.ts', status: 'added' });
    expect(tree[1]).toMatchObject({ name: 'del.ts', status: 'deleted' });
  });

  it('groups files sharing a common directory', () => {
    const tree = buildFileTree(
      [file('src/a.ts'), file('src/b.ts')],
      [],
      new Map(),
    );
    expect(tree).toHaveLength(1);
    expect(tree[0].children).toHaveLength(2);
  });

  it('handles deeply nested paths', () => {
    const tree = buildFileTree([file('a/b/c/d/e.ts')], [], new Map());
    let node = tree[0];
    for (const name of ['a', 'b', 'c', 'd']) {
      expect(node).toMatchObject({ name, type: 'directory' });
      node = node.children![0];
    }
    expect(node).toMatchObject({ name: 'e.ts', type: 'file' });
  });
});
