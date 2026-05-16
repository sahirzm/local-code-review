import type { FileChange, FileTreeNode } from '../../../shared/types.js';

interface TreeBuildNode {
  name: string;
  path: string;
  file?: FileChange;
  children: Map<string, TreeBuildNode>;
}

export function buildFileTree(
  files: FileChange[],
  reviewedFiles: string[],
  commentCounts: Map<string, number>,
): FileTreeNode[] {
  const reviewedSet = new Set(reviewedFiles);
  const root: Map<string, TreeBuildNode> = new Map();

  for (const file of files) {
    const parts = file.path.split('/');
    let current = root;

    for (let i = 0; i < parts.length; i++) {
      const name = parts[i];
      const path = parts.slice(0, i + 1).join('/');
      const isLeaf = i === parts.length - 1;

      let node = current.get(name);
      if (!node) {
        node = { name, path, children: new Map() };
        current.set(name, node);
      }
      if (isLeaf) {
        node.file = file;
      }
      current = node.children;
    }
  }

  return sortTree(convertNodes(root, reviewedSet, commentCounts));
}

function convertNodes(
  map: Map<string, TreeBuildNode>,
  reviewed: Set<string>,
  commentCounts: Map<string, number>,
): FileTreeNode[] {
  const result: FileTreeNode[] = [];
  for (const node of map.values()) {
    if (node.file && node.children.size === 0) {
      result.push({
        name: node.name,
        path: node.file.path,
        type: 'file',
        status: node.file.status,
        additions: node.file.additions,
        deletions: node.file.deletions,
        isReviewed: reviewed.has(node.file.path),
        commentCount: commentCounts.get(node.file.path) ?? 0,
      });
    } else {
      result.push({
        name: node.name,
        path: node.path,
        type: 'directory',
        children: convertNodes(node.children, reviewed, commentCounts),
        isReviewed: false,
        commentCount: 0,
      });
    }
  }
  return result;
}

function sortTree(nodes: FileTreeNode[]): FileTreeNode[] {
  nodes.sort((a, b) => {
    if (a.type !== b.type) return a.type === 'directory' ? -1 : 1;
    return a.name.localeCompare(b.name);
  });
  for (const node of nodes) {
    if (node.children) node.children = sortTree(node.children);
  }
  return nodes;
}
