import type { ParsedFileDiff } from '../../../shared/types.js';
import { FileDiff, type PierreViewType } from './FileDiff.js';
import type { ShikiThemePair } from './diff/shikiTheme.js';

interface DiffViewProps {
  files: ParsedFileDiff[] | null;
  /** Index of the file to display; the view shows one file at a time. */
  currentIndex: number;
  viewType: PierreViewType;
  themeType: 'dark' | 'light';
  syntaxTheme: ShikiThemePair;
  /** Diff font size in px; drives a remount so pierre re-measures its layout. */
  fontSize: number;
  /** Diff line-height ratio; also part of the layout remount key. */
  lineHeight: number;
  activeCommentId?: string | null;
  scrollDirection?: 'forward' | 'backward' | null;
}

export function DiffView({
  files,
  currentIndex,
  viewType,
  themeType,
  syntaxTheme,
  fontSize,
  lineHeight,
  activeCommentId,
  scrollDirection,
}: DiffViewProps): React.JSX.Element {
  if (files === null) {
    return (
      <div className="diff-view skeleton" role="status" aria-label="Loading diffs">
        <div className="skeleton-line" />
        <div className="skeleton-line" />
        <div className="skeleton-line" />
        <div className="skeleton-line" />
      </div>
    );
  }

  if (files.length === 0) {
    return <p className="empty-state">No files changed</p>;
  }

  const file = files[Math.max(0, Math.min(files.length - 1, currentIndex))];

  return (
    <div className="diff-view-scroll">
      {/* Keying on the file path resets FileDiff's per-file state (open comment
          form, collapse) when navigating between files. The font-size /
          line-height are folded into the key too: @pierre/diffs measures its
          row layout once and caches it inside its shadow DOM, so a CSS-var
          change alone doesn't resize the rendered diff. Remounting on a font
          change forces pierre to re-measure at the new size. */}
      <FileDiff
        key={`${file.newPath || file.oldPath}:${fontSize}:${lineHeight}`}
        file={file}
        viewType={viewType}
        themeType={themeType}
        syntaxTheme={syntaxTheme}
        activeCommentId={activeCommentId}
        scrollDirection={scrollDirection}
      />
    </div>
  );
}
