import { useState, useMemo, useCallback } from 'react';
import type { FileChange, FileTreeNode } from '../../../shared/types.js';
import { buildFileTree } from '../utils/build-file-tree.js';
import { useReviewStore } from '../hooks/useReviewStore.js';

type StatusFilter = 'all' | FileChange['status'];
type QuickFilter = 'has-comments' | 'needs-review';

interface SidebarProps {
  files: FileChange[];
  onFileClick: (filePath: string) => void;
  activeFile?: string;
}

export function Sidebar({ files, onFileClick, activeFile }: SidebarProps): React.JSX.Element {
  const [collapsed, setCollapsed] = useState(false);
  const [search, setSearch] = useState('');
  const [statusFilter, setStatusFilter] = useState<StatusFilter>('all');
  const [quickFilters, setQuickFilters] = useState<Set<QuickFilter>>(new Set());

  const { comments, reviewedFiles } = useReviewStore();

  const commentCounts = useMemo(() => {
    const map = new Map<string, number>();
    for (const c of comments) {
      if (c.filePath) map.set(c.filePath, (map.get(c.filePath) ?? 0) + 1);
    }
    return map;
  }, [comments]);

  const filteredFiles = useMemo(() => {
    const reviewedSet = new Set(reviewedFiles);
    return files.filter((f) => {
      if (search && !f.path.toLowerCase().includes(search.toLowerCase())) return false;
      if (statusFilter !== 'all' && f.status !== statusFilter) return false;
      if (quickFilters.has('has-comments') && !commentCounts.has(f.path)) return false;
      if (quickFilters.has('needs-review') && reviewedSet.has(f.path)) return false;
      return true;
    });
  }, [files, search, statusFilter, quickFilters, commentCounts, reviewedFiles]);

  const tree = useMemo(
    () => buildFileTree(filteredFiles, reviewedFiles, commentCounts),
    [filteredFiles, reviewedFiles, commentCounts],
  );

  const reviewedCount = reviewedFiles.length;

  const toggleQuickFilter = useCallback((f: QuickFilter) => {
    setQuickFilters((prev) => {
      const next = new Set(prev);
      if (next.has(f)) next.delete(f); else next.add(f);
      return next;
    });
  }, []);

  if (collapsed) {
    return (
      <aside className="sidebar sidebar-collapsed" aria-label="File navigator">
        <button
          className="sidebar-toggle"
          onClick={() => setCollapsed(false)}
          type="button"
          aria-label="Expand sidebar"
          title="Expand sidebar"
        >
          ▶
        </button>
      </aside>
    );
  }

  const statuses: StatusFilter[] = ['all', 'added', 'modified', 'deleted', 'renamed'];

  return (
    <aside className="sidebar" aria-label="File navigator">
      <div className="sidebar-header">
        <span className="sidebar-title">
          Files {reviewedCount}/{files.length}
        </span>
        <button
          className="sidebar-toggle"
          onClick={() => setCollapsed(true)}
          type="button"
          aria-label="Collapse sidebar"
          title="Collapse sidebar"
        >
          ◀
        </button>
      </div>

      <div className="sidebar-filters">
        <input
          className="sidebar-search"
          type="search"
          placeholder="Filter files…"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          aria-label="Filter files by path"
        />
        <div className="sidebar-status-filters">
          {statuses.map((s) => (
            <button
              key={s}
              type="button"
              className={`filter-pill ${statusFilter === s ? 'filter-pill-active' : ''}`}
              onClick={() => setStatusFilter(s)}
            >
              {s === 'all' ? 'All' : s.charAt(0).toUpperCase() + s.slice(1)}
            </button>
          ))}
        </div>
        <div className="sidebar-quick-filters">
          <button
            type="button"
            className={`filter-pill ${quickFilters.has('has-comments') ? 'filter-pill-active' : ''}`}
            onClick={() => toggleQuickFilter('has-comments')}
          >
            💬 Has comments
          </button>
          <button
            type="button"
            className={`filter-pill ${quickFilters.has('needs-review') ? 'filter-pill-active' : ''}`}
            onClick={() => toggleQuickFilter('needs-review')}
          >
            ○ Needs review
          </button>
        </div>
      </div>

      <nav className="sidebar-tree" aria-label="File tree">
        {tree.length === 0 ? (
          <p className="sidebar-empty">No files match</p>
        ) : (
          <FileTree nodes={tree} onFileClick={onFileClick} activeFile={activeFile} depth={0} />
        )}
      </nav>
    </aside>
  );
}

interface FileTreeProps {
  nodes: FileTreeNode[];
  onFileClick: (filePath: string) => void;
  activeFile?: string;
  depth: number;
}

function FileTree({ nodes, onFileClick, activeFile, depth }: FileTreeProps): React.JSX.Element {
  return (
    <ul className="tree-list" role="tree">
      {nodes.map((node) => (
        <TreeNode key={node.path} node={node} onFileClick={onFileClick} activeFile={activeFile} depth={depth} />
      ))}
    </ul>
  );
}

interface TreeNodeProps {
  node: FileTreeNode;
  onFileClick: (filePath: string) => void;
  activeFile?: string;
  depth: number;
}

function TreeNode({ node, onFileClick, activeFile, depth }: TreeNodeProps): React.JSX.Element {
  const [expanded, setExpanded] = useState(true);

  if (node.type === 'directory') {
    return (
      <li className="tree-item tree-dir" role="treeitem" aria-expanded={expanded}>
        <button
          className="tree-dir-btn"
          onClick={() => setExpanded((e) => !e)}
          type="button"
          style={{ paddingLeft: `${depth * 12 + 4}px` }}
        >
          <span className="tree-icon" aria-hidden="true">{expanded ? '▾' : '▸'}</span>
          <span className="tree-dir-name">{node.name}/</span>
        </button>
        {expanded && node.children && (
          <FileTree nodes={node.children} onFileClick={onFileClick} activeFile={activeFile} depth={depth + 1} />
        )}
      </li>
    );
  }

  const isActive = activeFile === node.path;
  const statusColors: Record<string, string> = {
    added: '#3fb950',
    modified: '#d29922',
    deleted: '#f85149',
    renamed: '#388bfd',
    copied: '#388bfd',
  };

  return (
    <li className={`tree-item tree-file ${isActive ? 'tree-file-active' : ''}`} role="treeitem">
      <button
        className="tree-file-btn"
        onClick={() => onFileClick(node.path)}
        type="button"
        style={{ paddingLeft: `${depth * 12 + 4}px` }}
        aria-current={isActive ? 'true' : undefined}
      >
        <span
          className="tree-status-dot"
          style={{ background: statusColors[node.status ?? 'modified'] }}
          aria-label={node.status}
        />
        <span className="tree-file-name">{node.name}</span>
        {node.isReviewed && <span className="tree-reviewed" aria-label="Reviewed">✓</span>}
        {node.commentCount > 0 && (
          <span className="tree-comment-count" aria-label={`${node.commentCount} comments`}>
            {node.commentCount}
          </span>
        )}
        <span className="tree-line-counts">
          <span className="additions">+{node.additions ?? 0}</span>
          <span className="deletions">-{node.deletions ?? 0}</span>
        </span>
      </button>
    </li>
  );
}
