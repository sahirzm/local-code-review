import { useRef, useCallback, useImperativeHandle, forwardRef } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import type { ParsedFileDiff } from '../../../shared/types.js';
import { FileDiff, type PierreViewType } from './FileDiff.js';
import type { ShikiThemePair } from './diff/shikiTheme.js';

interface DiffViewProps {
  files: ParsedFileDiff[] | null;
  viewType: PierreViewType;
  themeType: 'dark' | 'light';
  syntaxTheme: ShikiThemePair;
  activeCommentId?: string | null;
  scrollDirection?: 'forward' | 'backward' | null;
}

export interface DiffViewHandle {
  scrollToFile: (filePath: string) => void;
}

export const DiffView = forwardRef<DiffViewHandle, DiffViewProps>(function DiffView(
  { files, viewType, themeType, syntaxTheme, activeCommentId, scrollDirection },
  ref,
) {
  const parentRef = useRef<HTMLDivElement>(null);

  const virtualizer = useVirtualizer({
    count: files?.length ?? 0,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 400,
    overscan: 3,
  });

  const scrollToFile = useCallback(
    (filePath: string) => {
      const idx = files?.findIndex((f) => (f.newPath || f.oldPath) === filePath) ?? -1;
      if (idx < 0) return;
      // Always drive the scroll through the virtualizer. The previous guard
      // (only scroll when the row wasn't already in the DOM) made tree clicks
      // no-op whenever the target was rendered in the overscan window.
      virtualizer.scrollToIndex(idx, { align: 'start' });
    },
    [files, virtualizer],
  );

  useImperativeHandle(ref, () => ({ scrollToFile }), [scrollToFile]);

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

  return (
    <div ref={parentRef} className="diff-view-scroll">
      <div
        className="diff-view-inner"
        style={{ height: `${virtualizer.getTotalSize()}px`, position: 'relative' }}
      >
        {virtualizer.getVirtualItems().map((virtualItem) => {
          const file = files[virtualItem.index];
          return (
            <div
              key={virtualItem.key}
              data-index={virtualItem.index}
              ref={virtualizer.measureElement}
              style={{
                position: 'absolute',
                top: 0,
                left: 0,
                width: '100%',
                transform: `translateY(${virtualItem.start}px)`,
              }}
            >
              <FileDiff
                file={file}
                viewType={viewType}
                themeType={themeType}
                syntaxTheme={syntaxTheme}
                activeCommentId={activeCommentId}
                scrollDirection={scrollDirection}
              />
            </div>
          );
        })}
      </div>
    </div>
  );
});
