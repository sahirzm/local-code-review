import type { ParsedFileDiff } from '../../../shared/types.js';
import { FileDiff, type PierreViewType } from './FileDiff.js';
import type { ShikiThemePair } from './diff/shikiTheme.js';

/** Cheap, stable 32-bit string hash (djb2) of a patch, used only as part of a
 *  React remount key so a refresh with new content re-renders the diff. */
function hashPatch(patch: string): number {
  let h = 5381;
  for (let i = 0; i < patch.length; i++) {
    h = ((h << 5) + h + patch.charCodeAt(i)) | 0;
  }
  return h >>> 0;
}

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

  // A content signature so a refresh that changes a file's diff (same path)
  // remounts the surface. @pierre/diffs caches rendered output in a
  // module-level worker AST cache keyed by patch hash; without a remount the
  // persisted FileDiff instance can keep showing the pre-refresh diff.
  const contentSig = hashPatch(file.rawPatch);

  return (
    <div className="diff-view-scroll">
      {/* Keying on the file path resets FileDiff's per-file state (open comment
          form, collapse) when navigating between files. The font-size /
          line-height are folded into the key too: @pierre/diffs measures its
          row layout once and caches it inside its shadow DOM, so a CSS-var
          change alone doesn't resize the rendered diff. Remounting on a font
          change forces pierre to re-measure at the new size. The content
          signature remounts on a refresh that changes this file's diff. */}
      <FileDiff
        key={`${file.newPath || file.oldPath}:${fontSize}:${lineHeight}:${contentSig}`}
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
